FROM rust as base
WORKDIR /build
COPY Cargo* ./
COPY src ./src

RUN apt-get update -y
RUN apt-get upgrade -y
RUN apt-get install -y cmake 

RUN curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o /bin/yt-dlp
RUN chmod a+rx /bin/yt-dlp 

RUN cargo build --release
RUN cp -R /build/target/release /app

# ENVIRONMENT VARIABLES
WORKDIR /app
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1
ENV DISCORD_CACHE_DIR=/audio

# VOLUME
VOLUME /audio

CMD ["./discord_music_player"]
