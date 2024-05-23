#!/bin/sh
image_name="untagged6785/ha-mitaffald"
container_name="ha-mitaffald"

docker pull $image_name:latest

echo "Stopping container $container_name"
docker stop $container_name || true

echo "Removing container $container_name"
docker rm $container_name || true

echo "Starting new container"
docker run -d --name $container_name -e affaldvarme_address_id -e mqtt_username -e mqtt_password --restart=on-failure:5 $image_name:latest

# run this via /etc/config/crontab and not via crontab -e, because QNAP...
# crontab /etc/config/crontab && /etc/init.d/crond.sh restart