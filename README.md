## Remores-dl

Simple CLI tool to download canvas submissions for students that have
booked a time slot on REMORES. Perhaps slightly useful for TAs in
DD2350 at KTH, and useful for no one else.

It scrapes the REMORES website, and finds time slots that have been
booked by students. It then tries to find matching submissions on
canvas, and downloads them to a specified directory.

You will need a `CANVAS_API_TOKEN` environment variable set to a valid
Canvas API token. You can generate one by going to `Account ->
Settings -> Approved Integrations` on Canvas.

### Installation

If you have a working Rust compiler (if not see [here](https://rustup.rs/)), 
you can install by simply running:

```
cargo install --git https://github.com/adriansalamon/remores-dl
```

Otherwise, you can grab a precompiled binary from 
[releases](https://github.com/adriansalamon/remores-dl/releases/latest)
for your architecture. Let me know if you need more architectures added.

### Usage

```bash
remores-dl help
```

List courses:

```bash
remores-dl courses
```

List assignments for a course:

```bash
remores-dl assignments <course_id>
```

Download submissions for an assignment, for students that have booked
a time slot with your KTH ID:

```bash
remores-dl download --kth-id <kth_id> --repo <remores_repo_name> --course <id> --assignment <id>
```

### Building

You of course also build from source. Clone the repo and run:

```bash
cargo build --release
```

The binary will be located at `target/release/remores-dl`, and can be
moved to a directory in your PATH.
