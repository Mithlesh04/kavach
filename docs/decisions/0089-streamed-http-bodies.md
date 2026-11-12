# 0089. HTTP bodies whole when small, read as they come when large

- **Status:** Accepted
- **Date:** 2026-11-12

## Context

`http.serve` read every request body whole into `r.body`, and every response body was one `Bytes`. An upload of a large file or a download of one held all of it in memory, once per request. Rath, a web framework written in Kavach, asked (2026-11-12) for streamed request and response bodies, streamed multipart uploads, and files sent without being loaded.

A maintainer chose (2026-11-12) the first of the options put to them.

## Decision

- **Small request bodies stay whole** (spec 14.11): up to `ServeLimits.buffered_body` (1 MB), `r.body` holds the body, as before, so most handlers change nothing.
- **Large ones are read as they come:** a body larger than that, up to `max_body_bytes`, is left unread, and the handler reads it a piece at a time with `r.read_body() -> Bytes? ! NetError`. A body left partly unread closes the connection after the answer, so the next request can't start in the middle of it.
- **A response can stream:** `Response(…, stream: fs.chunks(…))` sends the reader's pieces as they are read, after `body`; `http.file(path)` builds one for a file.

## Consequences

- An upload or download of any size needs about one piece of memory.
- A streamed request can't be watched for the client leaving (its handler reads the connection), so a read in it fails instead.
- Multipart uploads read part by part come with `read_body`; `sendfile` for files, which skips copying through the program, is left for later.

## Alternatives considered

- **Every body streamed,** as Rust's hyper does: one model, but every handler that wants a body writes the reading loop.
- **A second serve function for streams:** two APIs to learn and keep in step.
