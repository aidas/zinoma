# buildy - An ultra-fast parallel build system for local iteration

**Warning: `buildy` is not yet ready for use**

`buildy` is very simple and configurable. It's meant to facilitate development by watching the file system and rebuilding as necessary.

The core of `buildy` is in its configuration file. Simply add a file named `buildy.yml` into the root of the project.

This configuration file tells `buildy` exactly what to do.

## buildy.yml spec
```yaml
server_deps:
  input_paths:
    - server/package.json
    - server/yarn.lock
  build:
    - cd server && yarn install

server:
  dependencies:
    - server_deps
  input_paths:
    - server/src
  service: cd server && node src/server.js
```

The configuration file is composed of `target`s, which are an entity to build. Each `target` can define its `dependencies`, `input_paths` to watch, commands to `build`, and a `service` command to run after the build completes.

`target`s will be run in parallel as much as possible, waiting to start until their `dependencies` are built.

The commands in the `build` list under a target are run one after another in a shell working from the directory containing the `buildy.yml`.

The `target` will only be rebuilt if either no `input_paths` exist, or any of the contents of the `input_paths` have changed since the last build.

`buildy` maintains a list of checksums in a directory named `.buildy` next to the `buildy.yml`. This directory should be ignored in version control.

Once a `target` is built, its `service` will be run. The `service` command can continue running in the background, which is useful for running programs such as a web server.

Even after everything is built, `buildy` will watch all the paths in the `input_paths` for new changes in the background. When any change is detected, those `target`s are rebuilt. Children `target`s (those that have a dependency on the rebuilt `target`) will only be rerun if files in their `input_paths` changed as well.

Whenever a `target` is rebuilt, its `service` command is rerun as well, terminating any that may still be running.

## Usage

Install with `cargo install buildy`.

Then, simply run `buildy <TARGETS>...` in a directory with `buildy.yml`.

e.g.

```shell script
buildy target_1 target_2
```

## Known Issues

* `service` commands that should be restarted when input paths change need to use `exec` or else they won't terminate properly.
* Output is not very pretty
* No unit tests
