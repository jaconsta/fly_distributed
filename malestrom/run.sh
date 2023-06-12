#!/bin/bash

tag_name=jaconsta/malestrom:latest
name=fly_maelstrome
docker run -i -t -v $(pwd)/../target:/opt/target -p 8080:8080 --name $name $tag_name bash
