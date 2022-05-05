# Prometheus alertmanager discord bridge

This tool will proxy alertmanager alerts to a discord web hook.

Set the env var `DISCORD_WEBHOOK_URL` to the full discord web hook URL.

It is written in rust mostly as an experiment.

This project is highly inspired from the go implementation

<https://github.com/benjojo/alertmanager-discord>.
