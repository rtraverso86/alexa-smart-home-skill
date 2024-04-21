# alexa-smart-home-skill

Rust version of the Python script for the AWS Lambda used in the 
[Amazon Alexa Smart Home Skill](https://www.home-assistant.io/integrations/alexa.smart_home/#add-code-to-the-lambda-function)
integration guide for Home Assistant.

It supports the same fetures, despite using a slightly different set of
environment variables), but being natively compiled in Rust and not requiring
a Python runtime when the Lambda performs a cold start, it is faster
at both being initialized as well as at runtime.

## Build

Follow the steps indicated at [AWS Docs for Rust Lambdas](https://docs.aws.amazon.com/lambda/latest/dg/rust-package.html#rust-package-build)
to install `cargo-lambda`.

Then, from a local clone of the repository, type

```
cargo lambda build -r --output-format zip
```

The command wil produce the `target/lambda/alexa-smart-home-skill/bootstrap.zip` package.

## Deployment Steps

Follow the original [guide](https://www.home-assistant.io/integrations/alexa.smart_home/#add-code-to-the-lambda-function)
step by step, with these exception during the `ADD CODE TO THE LAMBDA FUNCTION` section:

1. When the guide tells you to select a Python `Runtime`, pick `Amazon Linux 2023`,
   `x86_64` instead. In the `Handler` field write `provided`.
2. When you're on the `Code source` tab, instead of copying the Python script click on
   the `Upload from` button, select `.zip file`, and upload the `bootstrap.zip` package
   previously compiled.
3. Under the `Configuration` tab, the `DEBUG` environment variable is not supported. Instead:
   * Optionally, use the `RUST_LOG` environment variable to set the logging level
     (`OFF`, `TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR`).
   * The optional `LONG_LIVED_ACCESS_TOKEN` environment variable is only used when `RUST_LOG`
     is at least `DEBUG`.
   * Optionally, set `RUST_BACKTRACE` to `1` for more detailed crash information when
     debugging.

## Environment Variables

| NAME | REQUIRED | DESCRIPTION |
| :--- | :------: | :---------- |
| `BASE_URL` | Yes | Public address of your Home Assistant interface, e.g. `https://ha.example.com` |
| `RUST_LOG` | No | Sets the logging level and, at `DEBUG` or higher, enables the usage of `LONG_LIVED_ACCESS_TOKEN`. See [docs.rs](https://docs.rs/env_logger/latest/env_logger/#enabling-logging) for more details. |
| `LONG_LIVED_ACCESS_TOKEN` | No | Useful for debug and testing from the AWS Lambda Console, specify an access token generated within Home Assistant to be used for authentication. It is ignored unless `RUST_LOG` is set at `DEBUG` or higher. |
| `NOT_VERIFY_SSL` | No | `true` / `false` (default) deciding whether the SSL certificate at your `BASE_URL` should skip verification, in example when you're using a self-signed certificate. |
| `RUST_BACKTRACE` | No | To be set to `1` to enable the Lambda to log a backtrace in case of crashes. Useful for debugging and for reporting issues. |


## References

* https://www.home-assistant.io/integrations/alexa.smart_home/#add-code-to-the-lambda-function
* https://gist.github.com/jjmerri/fe22ca8ee0e80805005d670ac7f7818c
* https://docs.aws.amazon.com/lambda/latest/dg/lambda-rust.html