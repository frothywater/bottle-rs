use base64::{engine::general_purpose, Engine};
use rand::Rng;
use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use std::{
    num::ParseIntError,
    time::{SystemTime, UNIX_EPOCH},
};

use super::error::{Error, Result};

#[allow(unused)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    key_bytes: Vec<u8>,
    js_hash: String,
    row_index: usize,
    key_bytes_indices: Vec<usize>,
    svg_paths: Vec<String>,
    animation_key: String,
}

impl Transaction {
    const DEFAULT_KEYWORD: &str = "obfiowerehiring";
    const TOTAL_TIME: f64 = 4096.0;
    const ADDITIONAL_RANDOM_NUMBER: u8 = 3;
    const TWITTER_EPOCH_MS: u128 = 1_682_924_400u128 * 1000;

    pub async fn new(client: &Client) -> Result<Self> {
        // 1. Fetch and parse homepage
        let resp = client.get("https://x.com").send().await?;
        let body = resp.text().await?;

        // 2. Discover indices from ondemand JS
        let js_hash = Self::get_js_hash(&body)?;
        tracing::debug!("ondemand file hash: {}", js_hash);
        let (row_index, key_bytes_indices) = Self::get_indices(client, &js_hash).await?;
        tracing::debug!("row index: {}, key bytes indices: {:?}", row_index, key_bytes_indices);

        // 3. Extract & decode key
        let homepage = Html::parse_document(&body);
        let key = Self::get_key(&homepage)?;
        let key_bytes = general_purpose::STANDARD.decode(&key)?;
        tracing::debug!("twitter-site-verification: {}, bytes: {:?}", key, key_bytes);

        // 4–6. Compute animation key
        let svg_paths = Self::get_svg_paths(&homepage)?;
        let animation_key = Self::compute_animation_key(&svg_paths, &key_bytes, row_index, &key_bytes_indices)?;
        tracing::debug!("animation key: {}", animation_key);

        Ok(Self {
            key_bytes,
            js_hash,
            row_index,
            key_bytes_indices,
            svg_paths,
            animation_key,
        })
    }

    fn get_key(doc: &Html) -> Result<String> {
        let sel = Selector::parse(r#"meta[name="twitter-site-verification"]"#).unwrap();
        let element = doc.select(&sel).next().ok_or(Error::TransactionIDError(
            "Missing twitter-site-verification meta".to_string(),
        ))?;
        let key = element
            .value()
            .attr("content")
            .ok_or(Error::TransactionIDError("Meta tag has no content".to_string()))?;
        Ok(key.to_string())
    }

    fn get_js_hash(homepage_body: &str) -> Result<String> {
        let re_file = Regex::new(r#"['"]ondemand\.s['"]\s*:\s*['"]([\w]+)['"]"#)?;
        let hash = re_file
            .captures(homepage_body)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .ok_or(Error::TransactionIDError("ondemand file hash not found".to_string()))?;
        Ok(hash)
    }

    async fn get_indices(client: &Client, js_hash: &str) -> Result<(usize, Vec<usize>)> {
        let url = format!(
            "https://abs.twimg.com/responsive-web/client-web/ondemand.s.{}a.js",
            js_hash
        );
        let resp = client.get(&url).send().await?.error_for_status()?;
        let text = resp.text().await?;

        // extract indices
        let re_idx = Regex::new(r#"\(\w\[(\d{1,2})\],\s*16\)"#)?;
        let mut indices = re_idx
            .captures_iter(&text)
            .filter_map(|c| c.get(1))
            .map(|m| m.as_str().parse::<usize>())
            .collect::<std::result::Result<Vec<_>, ParseIntError>>()?;
        tracing::debug!("indices: {:?}", indices);

        if indices.len() < 2 {
            return Err(Error::TransactionIDError("Not enough key-byte indices".to_string()));
        }
        let row = indices.remove(0);
        Ok((row, indices))
    }

    fn get_svg_paths(homepage: &Html) -> Result<Vec<String>> {
        let svg_sel = Selector::parse(r#"[id^="loading-x-anim"]"#).unwrap();
        let path_sel = Selector::parse("path").unwrap();
        let mut paths = Vec::new();

        for svg in homepage.select(&svg_sel) {
            let mut d_values = svg.select(&path_sel).filter_map(|p| p.value().attr("d"));

            if let Some(second_d) = d_values.nth(1) {
                paths.push(second_d.to_string());
            } else {
                return Err(Error::TransactionIDError(
                    "SVG frame missing a second <path>".to_string(),
                ));
            }
        }
        tracing::debug!("svg paths: {:?}", paths);
        if paths.len() != 4 {
            return Err(Error::TransactionIDError("Expected 4 animation frames".to_string()));
        }
        Ok(paths)
    }

    fn compute_animation_key(
        svg_paths: &[String],
        key_bytes: &[u8],
        row_index: usize,
        key_bytes_indices: &[usize],
    ) -> Result<String> {
        // Extract animation details
        let (cubic, target_time, from_color, to_color, from_rot, to_rot) =
            Self::extract_animation(svg_paths, key_bytes, row_index, key_bytes_indices)?;

        // Animate to get color and rotation
        let (color, rotation) = Self::animate(&cubic, target_time, &from_color, &to_color, from_rot, to_rot)?;

        // Build hex parts
        let hex_key = Self::build_hex(&color, rotation);
        Ok(hex_key)
    }

    fn parse_path_data(path_d: &str) -> Vec<Vec<i32>> {
        let after_c = path_d.splitn(2, 'C').nth(1).unwrap_or("");
        let segs = after_c.split('C');
        let re_nums = Regex::new(r"[^\d]+").unwrap();
        segs.map(|seg| {
            re_nums
                .replace_all(seg, " ")
                .split_whitespace()
                .filter_map(|s| s.parse::<i32>().ok())
                .collect()
        })
        .collect()
    }

    fn extract_animation(
        svg_paths: &[String],
        key_bytes: &[u8],
        row_index: usize,
        key_bytes_indices: &[usize],
    ) -> Result<(Cubic, f64, Vec<f64>, Vec<f64>, f64, f64)> {
        // Pick frame index
        let frame_selector = (key_bytes[5] as usize) % 4;
        let arr2d = Self::parse_path_data(&svg_paths[frame_selector]);
        tracing::debug!("frame selector: {}", frame_selector);
        tracing::debug!("2D array: {:?}", arr2d);

        // Pick row
        let arr_row_index = (key_bytes[row_index] % 16) as usize;
        let row = arr2d[arr_row_index].clone();
        tracing::debug!("2D array[{}]: {:?}", arr_row_index, row);

        // Colors & rotation
        let from_color = vec![row[0] as f64, row[1] as f64, row[2] as f64, 1.0];
        let to_color = vec![row[3] as f64, row[4] as f64, row[5] as f64, 1.0];
        let from_rot = 0.0;
        let to_rot = linear_mapping(row[6] as f64, 60.0, 360.0, true);
        tracing::debug!("color: {:?} -> {:?}", from_color, to_color);
        tracing::debug!("rotation: {} -> {}", from_rot, to_rot);

        // Bezier curve
        let curves: Vec<f64> = row[7..]
            .iter()
            .enumerate()
            .map(|(i, &v)| linear_mapping(v as f64, is_odd(i as f64), 1.0, false))
            .collect();
        let cubic = Cubic::new(curves);
        tracing::debug!("cubic curves: {:?}", cubic.curves);

        // Frame time
        let frame_bytes = key_bytes_indices.iter().map(|&i| (key_bytes[i] % 16) as f64);
        let frame_time: f64 = frame_bytes.clone().product();
        let target_time = frame_time / Self::TOTAL_TIME;
        tracing::debug!("frame time: {:?} -> {}", frame_bytes, frame_time);
        tracing::debug!("target time: {}", target_time);

        Ok((cubic, target_time, from_color, to_color, from_rot, to_rot))
    }

    fn animate(
        cubic: &Cubic,
        target_time: f64,
        from_color: &[f64],
        to_color: &[f64],
        from_rot: f64,
        to_rot: f64,
    ) -> Result<(Vec<f64>, f64)> {
        // Interpolate
        let t = cubic.get_value(target_time);
        let color = interpolate(&from_color, &to_color, t);
        let rotation = interpolate(&[from_rot], &[to_rot], t)[0];
        tracing::debug!("cubic value: {}", t);
        tracing::debug!("target color: {:?}", color);
        tracing::debug!("target rotation: {}", rotation);

        Ok((color, rotation))
    }

    fn build_hex(color: &[f64], rotation: f64) -> String {
        // Rotation matrix
        let matrix = convert_rotation_to_matrix(rotation);
        tracing::debug!("rotation matrix: {:?}", matrix);

        // Build hex parts
        let mut parts: Vec<String> = color[..3]
            .iter()
            .map(|&c| format!("{:x}", c.max(0.0).round() as i64))
            .collect();
        for value in &matrix {
            let rounded = ((value * 100.0).round() / 100.0).abs();
            let piece = float_to_hex(rounded);
            parts.push(piece);
        }
        parts.push("0".into());
        parts.push("0".into());
        tracing::debug!("hex parts: {:?}", parts);

        // Join and strip
        let joined = parts.join("");
        let re_strip = Regex::new(r"[.-]").unwrap();
        re_strip.replace_all(&joined, "").to_string()
    }

    pub fn generate_id(&self, method: &str, path: &str) -> Result<String> {
        // time bytes
        let now_ms = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
        let adjusted = ((now_ms - Self::TWITTER_EPOCH_MS) / 1000) as u64;
        let time_bytes: Vec<u8> = (0..4).map(|i| ((adjusted >> (8 * i)) & 0xFF) as u8).collect();
        tracing::debug!(
            "current time: {}, adjusted: {}, bytes: {:?}",
            now_ms,
            adjusted,
            time_bytes
        );

        // sha256
        let input = format!(
            "{}!{}!{}{}{}",
            method,
            path,
            adjusted,
            Self::DEFAULT_KEYWORD,
            self.animation_key
        );
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        let hash = hasher.finalize();
        let hash16 = &hash[..16];
        tracing::debug!("sha256 {} -> {}", input, hex::encode(hash));

        // Assemble payload: `{key_bytes}{time_bytes}{hash16}{num=3}`
        let mut payload = Vec::new();
        payload.extend(&self.key_bytes);
        payload.extend(&time_bytes);
        payload.extend(hash16);
        payload.push(Self::ADDITIONAL_RANDOM_NUMBER);
        tracing::debug!(
            "payload: key_bytes: {}, time_bytes: {}, hash16: {}, num: {}",
            hex::encode(&self.key_bytes),
            hex::encode(&time_bytes),
            hex::encode(hash16),
            Self::ADDITIONAL_RANDOM_NUMBER
        );

        // xor with random byte, final data: `{rand}{payload^rand}`
        let rand_byte: u8 = rand::rng().random();
        let xored: Vec<u8> = payload.iter().map(|b| b ^ rand_byte).collect();
        let mut final_bytes = Vec::with_capacity(1 + xored.len());
        final_bytes.push(rand_byte);
        final_bytes.extend(xored);
        tracing::debug!("rand num: {}, final: {}", rand_byte, hex::encode(&final_bytes));

        // base64, stripped suffix '='
        let mut txid = general_purpose::STANDARD.encode(&final_bytes);
        tracing::debug!("base64: {}", txid);
        while txid.ends_with('=') {
            txid.pop();
        }
        Ok(txid)
    }
}

/// Return -1.0 if odd, else 0.0
fn is_odd(n: f64) -> f64 {
    if (n as i64) % 2 != 0 {
        -1.0
    } else {
        0.0
    }
}

/// Solve helper: linear mapping & optional rounding or flooring
fn linear_mapping(val: f64, min: f64, max: f64, rounding: bool) -> f64 {
    let res = val * (max - min) / 255.0 + min;
    if rounding {
        res.floor()
    } else {
        // ensure two decimals
        (res * 100.0).round() / 100.0
    }
}

/// Linear interpolate two same-length slices
fn interpolate(from: &[f64], to: &[f64], f: f64) -> Vec<f64> {
    from.iter()
        .zip(to.iter())
        .map(|(&a, &b)| a * (1.0 - f) + b * f)
        .collect()
}

/// Build 2×2 rotation matrix [cos, -sin, sin, cos]
fn convert_rotation_to_matrix(deg: f64) -> [f64; 4] {
    let rad = deg.to_radians();
    [rad.cos(), -rad.sin(), rad.sin(), rad.cos()]
}

/// Convert float to hex-string (integer and fractional parts)
fn float_to_hex(x: f64) -> String {
    let int_part = x.trunc() as u64;
    let mut frac_part = x.fract();
    let mut result = if int_part == 0 {
        "0".to_string()
    } else {
        format!("{:x}", int_part)
    };

    if frac_part != 0.0 {
        result.push('.');
        while frac_part > 0.0 {
            frac_part *= 16.0;
            let digit = frac_part.trunc() as u8;
            result.push(std::char::from_digit(digit as u32, 16).unwrap());
            frac_part -= digit as f64;
            // Optional: break early if precision is enough
            if result.len() > 32 {
                // prevent infinite loop on float precision errors
                break;
            }
        }
    }

    result
}

/// Cubic Bezier inversion + evaluation
struct Cubic {
    curves: Vec<f64>,
}

impl Cubic {
    const EPS: f64 = 1e-6;

    fn new(curves: Vec<f64>) -> Self {
        Self { curves }
    }

    fn calculate(a: f64, b: f64, m: f64) -> f64 {
        3.0 * a * (1.0 - m).powi(2) * m + 3.0 * b * (1.0 - m) * m.powi(2) + m.powi(3)
    }

    fn get_value(&self, time: f64) -> f64 {
        let (mut start, mut end) = (0.0, 1.0);

        if time <= 0.0 {
            let grad = if self.curves[0] > 0.0 {
                self.curves[1] / self.curves[0]
            } else if self.curves[1] == 0.0 && self.curves[2] > 0.0 {
                self.curves[3] / self.curves[2]
            } else {
                0.0
            };
            return grad * time;
        }

        if time >= 1.0 {
            let grad = if self.curves[2] < 1.0 {
                (self.curves[3] - 1.0) / (self.curves[2] - 1.0)
            } else if self.curves[2] == 1.0 && self.curves[0] < 1.0 {
                (self.curves[1] - 1.0) / (self.curves[0] - 1.0)
            } else {
                0.0
            };
            return 1.0 + grad * (time - 1.0);
        }

        let mut mid = 0.0;
        while (end - start) > Self::EPS {
            mid = (start + end) / 2.0;
            let x_est = Self::calculate(self.curves[0], self.curves[2], mid);
            if (time - x_est).abs() < Self::EPS {
                return Self::calculate(self.curves[1], self.curves[3], mid);
            }
            if x_est < time {
                start = mid;
            } else {
                end = mid;
            }
        }
        Self::calculate(self.curves[1], self.curves[3], mid)
    }
}
