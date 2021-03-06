# How to debug

It's recommended to use [VS Code] for debugging.

There are two folders which contains settings regarding VS Code:

* [.devcontainer](./.devcontainer) contains settings for
  [VS Code Remote Containers]
* [.vscode](./.vscode) contains basic settings

Before starting to debug using VS Code Remote Containers, you need to create
`Dockerfile` in the `.devcontainer` folder with the following command:

```shell
./docker/dockerfile-gen -d amd64 >.devcontainer/Dockerfile
```

create `mirakurun.openapi.json` in the project root folder with the following
command:

```shell
./scripts/mirakurun-openapi-json -c 2.14.0 >mirakurun.openapi.json
```

and then create `.devcontainer/config.yml`:

```shell
vi .devcontainer/config.yml
```

The following 3 configurations have been defined in `.vscode/launch.json`:

* Debug
* Debug w/ child processes (Debug + log messages from child processes)
* Debug unit tests

`SIGPIPE` never stops the debugger.  See `./vscode/settings.json`.

[VS Code]: https://code.visualstudio.com/
[VS Code Remote Containers]: https://code.visualstudio.com/docs/remote/containers
