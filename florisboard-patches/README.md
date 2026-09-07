## patched FlorisBoard

This directory contains a patch for building [FlorisBoard](https://github.com/florisboard/florisboard) with MPClipboard integration.

The latest APK file is always attached [to the latest release](https://github.com/iliabylich/mpclipboard/releases/tag/latest).

## build process

Release builds of Android apps require a signing key (can be self-signed).

```sh
keytool -genkey -v -keystore release-key.jks -keyalg RSA -keysize 2048 -validity 10000 -alias release
```

Once it's generated **make sure to save it**.

[Mise](https://mise.jdx.dev/) is the tool that is used to download and setup all dependencies, however we expect you to have Rust and Cargo installed globally.

First make sure to get all required dependencies:

```sh
mise setup:rust-target # to install Rust target
mise setup:cargo-ndk   # to install cargo-ndk (uses cargo-binstall under the hood)
mise setup:android-sdk # to install Java/Gradle/other build tools
```

Then to build the app run this (it will prompt you for the password that you entered previously)

```sh
mise build:florisboard /path/to/florisboard-release-key.jks
```

This will:

+ Build Rust part of MPClipboard
+ Build Kotlin wrapper of MPClipboard
+ Apply a patch with MPClipboard integration on Florisboard source code
+ Build Florisboard itself

It takes some time to build it, but once it's done the APK file will be available at:

```
florisboard/app/build/outputs/apk/release/app-release.apk
```
