use crate::response::GraphqlResponse;
use crate::result::{TimelineResult, Tweet, User};

fn read_response(endpoint: &str) -> GraphqlResponse {
    let path = std::fs::read_dir("log")
        .unwrap()
        .find(|f| f.as_ref().unwrap().path().to_str().unwrap().contains(endpoint))
        .unwrap()
        .unwrap()
        .path();
    let file = std::fs::File::open(path).unwrap();
    let response: GraphqlResponse = serde_json::from_reader(file).unwrap();
    response
}

#[test]
fn test_parse_user_by_id() {
    let response = read_response("UserByRestId");
    let _user: User = response.try_into().unwrap();
}

#[test]
fn test_parse_users_by_ids() {
    let response = read_response("UsersByRestIds");
    let _users: Vec<User> = response.try_into().unwrap();
}

#[test]
fn test_parse_tweet_by_id() {
    let response = read_response("TweetResultByRestId");
    let _tweet: Tweet = response.try_into().unwrap();
}

#[test]
fn test_parse_user_tweets() {
    let response = read_response("UserTweets");
    let _timeline: TimelineResult = response.try_into().unwrap();
}

#[test]
fn test_parse_user_media() {
    let response = read_response("UserMedia");
    let _timeline: TimelineResult = response.try_into().unwrap();
}

#[test]
fn test_parse_likes() {
    let response = read_response("Likes");
    let _timeline: TimelineResult = response.try_into().unwrap();
}

#[test]
fn test_parse_followers() {
    let response = read_response("Followers");
    let _timeline: TimelineResult = response.try_into().unwrap();
}

#[test]
fn test_parse_following() {
    let response = read_response("Following");
    let _timeline: TimelineResult = response.try_into().unwrap();
}
