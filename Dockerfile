FROM rust
WORKDIR /app
COPY Cargo* ./
COPY src ./src

RUN apt-get update -y
RUN apt-get upgrade -y
RUN apt-get install -y cmake 

RUN cargo install --path .
RUN cargo build --release

# ENVIRONMENT VARIABLES
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1
ENV AUDIO_FILES_PATH=/audio
ENV SAVED_QUEUES_PATH=/audio

CMD ["./target/release/discord_music_player"]