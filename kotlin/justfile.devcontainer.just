gradle := f"./gradlew --warning-mode \"all\" --console \"{{ if lowercase(env('CI', '')) == 'true' { 'colored' } else { 'verbose' } }}\" --stacktrace"

import "../ci/just/all.inc.just"
import "../ci/just/conf.inc.just"
import "../ci/just/devcont-inside.inc.just"
import "../ci/just/dir-root-plus-1.inc.just"
import "../ci/just/dir-kotlin.inc.just"
import "../ci/just/common-kotlin.inc.just"

[doc("[D] Run Gradle with specified arguments")]
gradle *args:
    @./gradlew "${@}"

[doc("[D] Run ADB with specified arguments")]
adb *args:
    @adb "${@}"

[doc("[D] Run ADB Enhanced with specified arguments")]
adbe *args:
    @adbe "${@}"

[doc("[D] Disconnect ADB from all Android devices")]
adb-disconnect-all:
    adb disconnect
    adb kill-server

[doc("[D] Run debug Android APK using ADB over network")]
apk:
    -adb uninstall "{{ apk_app }}"
    {{ gradle }} installDebug
    adbe start "{{ apk_app }}"
    adb logcat

# TODO: adb logcat --pid $( sleep 1 && adb shell pidof -s "{{ apk_app }}" )

_apk-build:
    {{ gradle }} assembleDebug

[doc("[D] Run release Android APK using ADB over network")]
apk-release: apk-build-release
    -adb uninstall "{{ apk_app }}"
    adb install \
        -r -t -g \
        {{ kotlin_dir }}/app/build/outputs/apk/release/*-signed.apk
    adbe start "{{ apk_app }}"
    adb logcat

# TODO: adb logcat --pid $( sleep 1 && adb shell pidof -s "{{ apk_app }}" )

_apk-build-release:
    {{ gradle }} assembleRelease
    gmake \
        --file "../../../../../../ci/make/sign-apk.mk" \
        --directory "app/build/outputs/apk/release" \
        "{{ extension(apk_app) }}-release-signed.apk"

[doc("[D] Clean Kotlin/Android subproject")]
kotlin-clean:
    {{ gradle }} clean
