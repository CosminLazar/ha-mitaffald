#!/bin/sh

image_name="untagged6785/ha-mitaffald"
container_name="ha-mitaffald"

docker pull $image_name:latest

echo "Stopping container $container_name"
docker stop $container_name || true

echo "Removing container $container_name"
docker rm $container_name || true

echo "Starting new container"
docker run -d --name $container_name -e affaldvarme.address.id -e mqtt.username -e mqtt.password --restart=on-failure:5 $image_name:latest