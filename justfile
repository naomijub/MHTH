#!/usr/bin/env just --justfile

set shell := ["powershell.exe", "-c"]

# hello is recipe's name
hello:
  echo "Hello World!"

[windows]
console:
    Start-Process "http://127.0.0.1:7351"

[unix]
console:
    open "http://127.0.0.1:7351"

nakama-up:
    docker compose -f nakama/local-nakama.yml build --no-cache
    docker compose -f nakama/local-nakama.yml up --build --force-recreate 

nakama-down:
    docker compose -f nakama/local-nakama.yml down


server-up:
    docker compose build --no-cache
    docker compose up --build --force-recreate 

server-down:
    docker compose down