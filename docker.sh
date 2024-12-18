#!/bin/bash
docker build -t vnix .
docker run --rm -it vnix
