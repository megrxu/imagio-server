# Imagio Server

Image resizing and cropping capabilities on demand.

## Run

- Using local filesystem:
  `cargo run --release -- --store <IMAGES_PATH> --cache <CACHE_PATH> fs serve`
- Using S3:
  ```bash
  cargo run --release -- --store <IMAGES_PATH> --cache <CACHE_PATH>         \
    --bucket <BUCKET> --region <REGION> --endpoint <ENDPOINT>               \
    --access-key-id <ACCESS_KEY_ID> --secret-access-key <SECRET_ACCESS_KEY> \
    s3 serve
  ```
