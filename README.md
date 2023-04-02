# nanum

_nanum_ is a file sharing service.

## Usage

Install [Trunk](https://trunkrs.dev/).

```
cd frontend
yarn
trunk build
cd ../backend
ALLOWED_EMAILS=<your@email.com> \
  GITHUB_CLIENT_ID=<github client id> \
  GIHTUB_CLIENT_SECRET=<github client secret> \
  JWT_SECRET=<your secret> \
  S3_BUCKET_NAME=<s3 bucket name> \
  cargo run
```

And open http://localhost:3000/ on your browser.

## License

`nanum` is licensed under the terms of the Apache 2.0 license.
See [LICENSE](./LICENSE) file for details.
