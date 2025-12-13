#!/bin/bash

set -e

RELEASE_DIR="target/release"
APP_DIR="$RELEASE_DIR/macos"
APP_NAME="Otty.app"
APP_PATH=$APP_DIR/$APP_NAME

codesign --remove-signature "$APP_PATH"
codesign --force --deep --sign - "$APP_PATH"
