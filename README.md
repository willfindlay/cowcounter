# cowcounter

This is a silly little program that keeps track of how many times you open a
window with a title that matches a regular expression. It serves that counter on
localhost so that you can import it as a browser source. The counter is also
written to disk on a configurable interval for persistence.

## How To Run It

1. Clone this repo.
2. `cd cowcounter`
3. `cargo run`

## Configuration

The following settings can be configured via environment variables:

- `COWCOUNTER_PORT`: port to serve the browser source (default `7788`)
- `COWCOUNTER_NAME_RE`: regex for application name (default `^RelicCardinal.exe$`)
- `COWCOUNTER_SAVEFILE`: pathname for save file (default `counter.txt`)
- `COWCOUNTER_BACKUP_INTERVAL`: backup interval for saving the counter to disk (default `5m`)

## Importing Into OBS

Create a browser source and set the URL to `localhost:7788` or whatever port you decided to use.

To configure the appearance of the counter, add custom CSS when setting up the browser source. For example, you can select the counter by its class:

```css
.counter {
    color: orange;
}
```