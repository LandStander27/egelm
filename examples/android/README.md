# Android example

Build the example APK and install it on a connected Android device:

```sh
cargo apk b -p android --lib
adb install target/debug/apk/android.apk
```

After installation, open **egelm example** from the device's app launcher.
