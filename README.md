# SnapRS

A Minecraft Server written in Rust with Performance in mind.

## Compiling

Because of how Minecraft handles chunks, we need a `blocks.json` containing all possible block states and their associated IDs from the targeted version.
This is currently expected to be in `generated/reports/blocks.json`. Getting this file is relatively easy, simply only requiring the following;

- Obtain `server.jar` for the target minecraft version
- Run `java -DbundlerMainClass="net.minecraft.data.Main" -jar server.jar --all`

This should create, among a few other misc folders, the `generated` folder, which contains info crucial for our BuildScript.
