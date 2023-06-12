# Maelstrom Commands for the exercises

Considering you are in /opt within the container

## (1) Echo

> maelstrom/maelstrom test -w echo --bin target/debug/fly_distributed --node-count 1 --time-limit 10

## (2) Unique IDs

> maelstrom/maelstrom test -w unique-ids --bin target/debug/fly_distributed --time-limit 30 --rate 1000 --node-count 3 --availability total --nemesis partition

## (3.a) Single node broadcast

> maelstrom/maelstrom test -w broadcast --bin target/debug/fly_distributed --time-limit 20 --node-count 1 --rate 1


