# Render Deployment Notes

## Deployment Model

Deploy as a Docker web service.

Render will build the image from the repository and inject environment variables at runtime.

## Required Environment Variables

- `APP_ENV=production`
- `APP_HOST=0.0.0.0`
- `APP_PORT`
- `RUST_LOG=info`
- `OPENAI_API_KEY`
- `ANTHROPIC_API_KEY`
- `GOOGLE_CLOUD_PROJECT`
- `GOOGLE_CLOUD_LOCATION`
- `GOOGLE_APPLICATION_CREDENTIALS_JSON`

## Secret Handling

Never commit:

- `.env`
- service account JSON files
- API keys
- generated credential files

For Vertex AI, store the service account JSON as `GOOGLE_APPLICATION_CREDENTIALS_JSON`.

If the backend needs a file path, it should write this env var to a temporary file at startup and point the SDK to that file.

## Future Docker Notes

The backend Docker image should:

- compile the Rust service in a builder stage
- copy only the final binary into the runtime stage
- avoid copying local `.env`
- expose the configured app port

The frontend can either:

- be served by the Rust backend as static files
- be deployed separately later
