import QtQuick
import QtQuick.Controls
import QtTest

TestCase {
    id: testCase
    name: "PhoneCamUiComprehensiveTest"
    width: 800
    height: 900
    visible: true

    Loader {
        id: mainLoader
        active: true
        source: "../ui/Main.qml"
    }

    property var win: mainLoader.item

    function getControl(testId) {
        var c = findChild(win, testId);
        if (!c && win && win.contentItem) {
            c = findChild(win.contentItem, testId);
        }
        return c;
    }

    function initTestCase() {
        tryCompare(mainLoader, "status", Loader.Ready, 3000);
        win = mainLoader.item;
        verify(win !== null, "ApplicationWindow loaded via relative path");
    }

    function test_1_testids_and_initial_state() {
        compare(win.objectName, "window-main");
        compare(win.title, "PhoneCam — Smartphone Virtual Webcam");
        compare(win.currentFacing, "back");
        compare(win.currentRes, "1920x1080");
        compare(win.isStreaming, false);

        var rearBtn = getControl("btn-camera-rear");
        verify(rearBtn !== null, "Found testId 'btn-camera-rear'");
        compare(rearBtn.checked, true, "Rear camera button initially checked");

        var frontBtn = getControl("btn-camera-front");
        verify(frontBtn !== null, "Found testId 'btn-camera-front'");
        compare(frontBtn.checked, false, "Front camera button initially unchecked");

        verify(getControl("combo-camera-lens") !== null, "Found testId 'combo-camera-lens'");
        verify(getControl("combo-resolution") !== null, "Found testId 'combo-resolution'");
        verify(getControl("combo-crop") !== null, "Found testId 'combo-crop'");
        verify(getControl("switch-flip-h") !== null, "Found testId 'switch-flip-h'");
        verify(getControl("switch-flip-v") !== null, "Found testId 'switch-flip-v'");
        verify(getControl("switch-invert") !== null, "Found testId 'switch-invert'");
        verify(getControl("switch-torch") !== null, "Found testId 'switch-torch'");
        verify(getControl("switch-preview") !== null, "Found testId 'switch-preview'");
        verify(getControl("btn-preview") !== null, "Found testId 'btn-preview'");
        verify(getControl("btn-stream-action") !== null, "Found testId 'btn-stream-action'");
        verify(getControl("btn-refresh") !== null, "Found testId 'btn-refresh'");
        verify(getControl("text-device-name") !== null, "Found testId 'text-device-name'");
        verify(getControl("text-stream-status") !== null, "Found testId 'text-stream-status'");
    }

    function test_2_simulate_click_camera_buttons() {
        var rearBtn = getControl("btn-camera-rear");
        var frontBtn = getControl("btn-camera-front");

        // Click front camera button
        mouseClick(frontBtn);
        compare(win.currentFacing, "front", "currentFacing switched to front");
        compare(frontBtn.checked, true, "Front button is checked");
        compare(rearBtn.checked, false, "Rear button is unchecked");

        // Click rear camera button
        mouseClick(rearBtn);
        compare(win.currentFacing, "back", "currentFacing switched back to rear");
        compare(rearBtn.checked, true, "Rear button is checked");
        compare(frontBtn.checked, false, "Front button is unchecked");
    }

    function test_3_simulate_click_switches() {
        var switchFlipH = getControl("switch-flip-h");
        var switchFlipV = getControl("switch-flip-v");
        var switchInvert = getControl("switch-invert");
        var switchTorch = getControl("switch-torch");
        var switchPreview = getControl("switch-preview");

        // Flip H
        compare(win.flipH, false);
        mouseClick(switchFlipH);
        compare(win.flipH, true, "Horizontal flip enabled via click");
        mouseClick(switchFlipH);
        compare(win.flipH, false, "Horizontal flip disabled via click");

        // Flip V
        compare(win.flipV, false);
        mouseClick(switchFlipV);
        compare(win.flipV, true, "Vertical flip enabled via click");
        mouseClick(switchFlipV);
        compare(win.flipV, false, "Vertical flip disabled via click");

        // Invert
        compare(win.invertColors, false);
        mouseClick(switchInvert);
        compare(win.invertColors, true, "Invert enabled via click");
        mouseClick(switchInvert);
        compare(win.invertColors, false, "Invert disabled via click");

        // Torch
        compare(win.torchOn, false);
        mouseClick(switchTorch);
        compare(win.torchOn, true, "Torch enabled via click");
        mouseClick(switchTorch);
        compare(win.torchOn, false, "Torch disabled via click");

        // Preview switch
        compare(win.showPreview, false);
        mouseClick(switchPreview);
        compare(win.showPreview, true, "Preview enabled via click");
        mouseClick(switchPreview);
        compare(win.showPreview, false, "Preview disabled via click");
    }

    function test_4_simulate_click_theme_swatches() {
        // Swatch 1: Cyber Emerald (#10B981)
        var swatch1 = getControl("btn-theme-swatch-1");
        verify(swatch1 !== null, "Found theme swatch 1");
        mouseClick(swatch1);
        compare(win.accentColor, "#10B981", "Accent switched to Cyber Emerald");

        // Swatch 2: Sky Blue (#38BDF8)
        var swatch2 = getControl("btn-theme-swatch-2");
        verify(swatch2 !== null, "Found theme swatch 2");
        mouseClick(swatch2);
        compare(win.accentColor, "#38BDF8", "Accent switched to Sky Blue");

        // Swatch 0: Omarchy Gold (#FFD60A)
        var swatch0 = getControl("btn-theme-swatch-0");
        verify(swatch0 !== null, "Found theme swatch 0");
        mouseClick(swatch0);
        compare(win.accentColor, "#FFD60A", "Accent switched to Omarchy Gold");
    }

    function test_5_combobox_and_preview_button() {
        var comboRes = getControl("combo-resolution");
        comboRes.currentIndex = 1;
        comboRes.activated(1);
        compare(win.currentRes, "1280x720", "Resolution updated to 720p");

        comboRes.currentIndex = 2;
        comboRes.activated(2);
        compare(win.currentRes, "3840x2160", "Resolution updated to 4K");

        var btnPreview = getControl("btn-preview");
        compare(win.showPreview, false);
        mouseClick(btnPreview);
        compare(win.showPreview, true, "Preview enabled via preview button click");
    }

    function test_6_bidirectional_camera_sync() {
        var rearBtn = getControl("btn-camera-rear");
        var frontBtn = getControl("btn-camera-front");
        var comboLens = getControl("combo-camera-lens");

        // Simulate multi-lens phone detected
        win.camerasList = [
            { id: 0, facing: "back", resolution: "1920x1080", zoom_range: "1.0x" },
            { id: 1, facing: "front", resolution: "1920x1080", zoom_range: "1.0x" },
            { id: 2, facing: "back", resolution: "3840x2160", zoom_range: "3.0x" }
        ];

        // Click front camera button
        mouseClick(frontBtn);
        compare(win.currentCameraId, 1, "Camera ID synced to front lens #1");
        compare(comboLens.currentIndex, 1, "Combobox index synced to #1");

        // Change combobox to telephoto lens #2 (facing=back)
        win.onCameraComboSelected(2);
        compare(win.currentCameraId, 2, "Camera ID updated to telephoto #2");
        compare(win.currentFacing, "back", "Facing auto-switched to back");
        compare(rearBtn.checked, true, "Rear button auto-checked");
        compare(frontBtn.checked, false, "Front button auto-unchecked");
    }

    function test_7_keyboard_shortcuts() {
        compare(win.isStreaming, false);
        keyClick(Qt.Key_Space);
    }
}
