FROM rust:bookworm as base
WORKDIR /build
COPY Cargo* ./
COPY src ./src

RUN apt-get update -y
RUN apt-get upgrade -y
RUN apt-get install -y cmake 

RUN cargo build --release

FROM debian:bookworm-slim as final

# INSTALL DEPENDENCIES

RUN apt-get update -y
RUN apt-get upgrade -y
RUN apt-get install -y curl python3 ffmpeg

RUN curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o /bin/yt-dlp
RUN chmod a+rx /bin/yt-dlp 

# COPY BINARY
COPY --from=base /build/target/release /app

# ENVIRONMENT VARIABLES
WORKDIR /app
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1
ENV DISCORD_CACHE_DIR=/audio

# VOLUME
VOLUME ${DISCORD_CACHE_DIR}

CMD ["./discord_music_player"]
