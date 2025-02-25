docker run \
    -v "./data:/app/data" \
    -v "/Users/cobalt/Pictures/Collection:/app/images" \
    -v "/Users/cobalt/Projects/misc/certs/ca-certificates.crt:/etc/ssl/certs/ca-certificates.crt" \
    -e SERVER_ADDRESS=0.0.0.0:6000 \
    -e DATABASE_URL=data/db.sqlite3 \
    -e IMAGE_DIR=images \
    -p 6000:6000 \
    --rm -it \
    bottle-server