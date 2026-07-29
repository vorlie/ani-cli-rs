# ani-cli-rs Documentation Site

This directory contains the source for the [ani-cli-rs documentation website](https://vorlie.github.io/ani-cli-rs).

The site is built with [MkDocs](https://www.mkdocs.org/) using the [Material theme](https://squidfunk.github.io/mkdocs-material/).

## Local Development

To preview the site locally, you'll need Python installed.

1. **Install dependencies:**
   ```sh
   pip install mkdocs-material
   ```

2. **Run the development server:**
   ```sh
   mkdocs serve
   ```
   This will start a local server at `http://127.0.0.1:8000/`.

## Deployment

The site is automatically deployed to GitHub Pages via a GitHub Action (`.github/workflows/pages.yml`) whenever changes are pushed to the `main` branch.

## Content

The documentation source files are located in the `docs/` directory. This is the canonical source for all user and contributor documentation; the GitHub Wiki has been retired.
