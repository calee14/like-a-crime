# like a crime

```pseudocode
(fast) fourier transform
over-engineered slop
```

<img src="demo.png" alt="demo image" height="500">

## install dependencies

### clone repo

```
git clone git@github.com:calee14/like-a-crime.git
```

### install dependencies

```bash
cargo build
```

## usage

```bash
cargo run -- -a <your-audio-file>.wav
cargo run -- -s # in the works 
```

### build for release

```bash
cargo build --release

./target/release/like-a-crime <your-audio-file>.wav
```
