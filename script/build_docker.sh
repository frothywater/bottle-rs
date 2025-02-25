docker build -t bottle-server .
docker save bottle-server -o target/bottle-server.tar
docker image prune --force