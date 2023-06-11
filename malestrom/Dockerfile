FROM debian:bullseye

RUN apt-get update
RUN apt-get install openjdk-17-jdk graphviz gnuplot curl ca-certificates wget bzip2 git -y
WORKDIR /opt
RUN wget https://github.com/jepsen-io/maelstrom/releases/download/v0.2.3/maelstrom.tar.bz2
RUN tar -xf ./maelstrom.tar.bz2

CMD ./maelstrom/maelstrom serve

