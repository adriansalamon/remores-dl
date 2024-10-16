## Remores-dl

Simple CLI tool to download canvas submissions for students
that have booked a time slot on REMORES.

It scrapes the REMORES website, and finds time slots that have
been booked by students. It then tries to find matching submissions
on canvas, and downloads them to a specified directory.

### Usage

```bash
remores-dl help
```

List courses:

```bash
remores-dl list-courses
```

List assignments for a course:

```bash
remores-dl list-assignments <course_id>
```

Download submissions for an assignment:

```bash
download --kth-id <kth_id> --repo <adk-mastarprov> --course <id> --assignment <id>
```

### Building

```bash
cargo build --release
```
