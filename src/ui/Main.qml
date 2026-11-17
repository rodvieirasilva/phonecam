import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ApplicationWindow {
    id: window
    objectName: "window-main"
    visible: true
    width: 700
    height: 820
    minimumWidth: 560
    minimumHeight: 680
    title: "PhoneCam — Smartphone Virtual Webcam"
    color: "#18181b"

    // Theme state (Tokyo Night / Omarchy Blue default)
    property string accentColor: "#7aa2f7"

    // Runtime state
    property int port: 41789
    property bool isStreaming: false
    property bool isDeviceConnected: false
    property string deviceName: "Detecting..."
    property string deviceSerial: ""
    property string batteryText: ""
    property var camerasList: []
    property var recentLogs: []

    // Synchronization guard to prevent event loops between buttons and combobox
    property bool isUpdatingCameraSelection: false

    // Wi-Fi ADB state
    property string wifiAddressText: ""
    property string wifiStatusMsg: ""
    property bool wifiStatusIsError: false
    property bool isWifiConnecting: false

    // Configuration model
    property string currentFacing: "back"
    property var currentCameraId: null
    property string currentRes: "1920x1080"
    property string currentAr: ""
    property string currentCrop: ""
    property int currentFps: 30
    property bool flipH: false
    property bool flipV: false
    property bool invertColors: false
    property bool showPreview: false
    property bool torchOn: false

    Component.onCompleted: {
        fetchStatus();
        pollTimer.start();
    }

    Timer {
        id: pollTimer
        interval: 2000
        repeat: true
        running: true
        onTriggered: {
            fetchStatus();
            fetchLogs();
        }
    }

    function apiCall(endpoint, method, payload, callback) {
        var xhr = new XMLHttpRequest();
        xhr.open(method, "http://127.0.0.1:" + port + endpoint, true);
        xhr.setRequestHeader("Content-Type", "application/json");
        xhr.onreadystatechange = function() {
            if (xhr.readyState === XMLHttpRequest.DONE) {
                var res = null;
                try {
                    res = JSON.parse(xhr.responseText);
                } catch (e) {
                    res = { success: xhr.status === 200, error: xhr.responseText || ("HTTP " + xhr.status) };
                }
                if (callback) callback(res, xhr.status);
            }
        };
        xhr.send(payload ? JSON.stringify(payload) : null);
    }

    function connectWifi(address) {
        var addr = (address !== undefined && address !== null && address !== "" ? address : wifiAddressText).trim();
        if (addr.length === 0) {
            wifiStatusMsg = "Please enter phone IP (e.g. 192.168.1.50:5555)";
            wifiStatusIsError = true;
            return;
        }
        isWifiConnecting = true;
        wifiStatusMsg = "Connecting to " + addr + "...";
        wifiStatusIsError = false;

        apiCall("/api/connect-wifi", "POST", { address: addr }, function(res, status) {
            isWifiConnecting = false;
            if (res && res.success) {
                wifiStatusMsg = "Connected: " + (res.message || addr);
                wifiStatusIsError = false;
                fetchStatus();
                fetchLogs();
            } else {
                wifiStatusMsg = (res && res.error) ? res.error : ("Failed to connect to " + addr);
                wifiStatusIsError = true;
            }
        });
    }

    function enableTcpip() {
        isWifiConnecting = true;
        wifiStatusMsg = "Enabling TCP/IP mode on port 5555...";
        wifiStatusIsError = false;

        apiCall("/api/enable-tcpip", "POST", null, function(res, status) {
            isWifiConnecting = false;
            if (res && res.success) {
                wifiStatusMsg = "Wireless mode enabled! You can now unplug the USB cable.";
                wifiStatusIsError = false;
                fetchStatus();
                fetchLogs();
            } else {
                wifiStatusMsg = (res && res.error) ? res.error : "Failed to enable TCP/IP mode. Is phone plugged via USB?";
                wifiStatusIsError = true;
            }
        });
    }

    function fetchStatus() {
        apiCall("/api/status", "GET", null, function(data, status) {
            if (!data || status !== 200) return;

            // Apply accent only if user hasn't explicitly customized it
            if (!userCustomizedAccent && data.accent_color) {
                accentColor = data.accent_color;
            }
            if (data.is_streaming !== undefined) {
                isStreaming = !!data.is_streaming;
            }

            if (data.device) {
                if (data.device.status === "Connected") {
                    isDeviceConnected = true;
                    deviceName = data.device.brand.toUpperCase() + " " + data.device.model;
                    deviceSerial = data.device.serial;
                    if (data.device.battery_percent !== null && data.device.battery_percent !== undefined) {
                        batteryText = data.device.battery_percent + "%" + (data.device.is_charging ? " ⚡" : "");
                    }
                } else if (data.device.status === "Unauthorized") {
                    isDeviceConnected = false;
                    deviceName = "Unauthorized (Tap Allow on Phone)";
                } else {
                    isDeviceConnected = false;
                    deviceName = "No smartphone detected";
                }
            }

            if (data.cameras && data.cameras.length > 0) {
                camerasList = data.cameras;
                syncCameraSelectionFromState();
            }

            if (data.config) {
                currentFacing = data.config.facing;
                currentCameraId = data.config.camera_id;
                currentRes = data.config.resolution;
                currentAr = data.config.camera_ar ? data.config.camera_ar : "";
                currentCrop = data.config.crop ? data.config.crop : "";
                currentFps = data.config.fps;
                flipH = data.config.flip_h;
                flipV = data.config.flip_v;
                invertColors = data.config.invert;
                torchOn = data.config.torch;
                showPreview = data.config.preview;
            }
        });
    }

    property bool userCustomizedAccent: false

    function setCustomAccent(colorHex) {
        userCustomizedAccent = true;
        accentColor = colorHex;
    }

    function fetchLogs() {
        apiCall("/api/logs", "GET", null, function(data, status) {
            if (Array.isArray(data)) {
                recentLogs = data;
            }
        });
    }

    // Bidirectional camera synchronization
    function syncCameraSelectionFromState() {
        if (isUpdatingCameraSelection || camerasList.length === 0) return;
        isUpdatingCameraSelection = true;

        if (currentCameraId !== null && currentCameraId !== undefined) {
            for (var i = 0; i < camerasList.length; i++) {
                if (camerasList[i].id === currentCameraId) {
                    cameraCombo.currentIndex = i;
                    currentFacing = camerasList[i].facing;
                    break;
                }
            }
        } else {
            // Find first camera matching currentFacing
            for (var j = 0; j < camerasList.length; j++) {
                if (camerasList[j].facing === currentFacing) {
                    cameraCombo.currentIndex = j;
                    currentCameraId = camerasList[j].id;
                    break;
                }
            }
        }

        isUpdatingCameraSelection = false;
    }

    function selectFacing(facing) {
        if (isUpdatingCameraSelection) return;
        isUpdatingCameraSelection = true;

        currentFacing = facing;

        // Auto-select the corresponding camera in the combobox
        var matchedIndex = -1;
        for (var i = 0; i < camerasList.length; i++) {
            if (camerasList[i].facing === facing) {
                matchedIndex = i;
                currentCameraId = camerasList[i].id;
                break;
            }
        }

        if (matchedIndex !== -1) {
            cameraCombo.currentIndex = matchedIndex;
            apiCall("/api/select-camera", "POST", { id: currentCameraId, facing: facing }, function() {
                fetchStatus();
            });
        } else {
            currentCameraId = null;
            apiCall("/api/set-facing", "POST", { facing: facing }, function() {
                fetchStatus();
            });
        }

        isUpdatingCameraSelection = false;
    }

    function onCameraComboSelected(index) {
        if (isUpdatingCameraSelection || index < 0 || index >= camerasList.length) return;
        isUpdatingCameraSelection = true;

        var selected = camerasList[index];
        currentFacing = selected.facing;
        currentCameraId = selected.id;

        apiCall("/api/select-camera", "POST", { id: selected.id, facing: selected.facing }, function() {
            fetchStatus();
        });

        isUpdatingCameraSelection = false;
    }

    function startStream() {
        var arVal = null;
        var cropVal = null;

        if (cropCombo.currentIndex === 1) { // 16:9
            arVal = "16:9";
        } else if (cropCombo.currentIndex === 2) { // 4:3
            arVal = "4:3";
        } else if (cropCombo.currentIndex === 3) { // 1:1 Square
            arVal = "1:1";
        } else if (cropCombo.currentIndex === 4) { // Custom 1080p center
            cropVal = "1920:1080:0:0";
        }

        var cfg = {
            video_nr: 10,
            card_label: "PhoneCam Virtual Camera",
            facing: currentFacing,
            camera_id: currentCameraId,
            resolution: currentRes,
            camera_ar: arVal,
            crop: cropVal,
            fps: currentFps,
            flip_h: flipH,
            flip_v: flipV,
            invert: invertColors,
            preview: showPreview,
            torch: torchOn,
            port: port
        };

        apiCall("/api/start", "POST", cfg, function(res) {
            fetchStatus();
        });
    }

    function stopStream() {
        apiCall("/api/stop", "POST", null, function(res) {
            fetchStatus();
        });
    }

    function toggleFlipH() {
        flipH = !flipH;
        apiCall("/api/toggle-flip-h", "POST", null, function() { fetchStatus(); });
    }

    function toggleFlipV() {
        flipV = !flipV;
        apiCall("/api/toggle-flip-v", "POST", null, function() { fetchStatus(); });
    }

    function toggleInvert() {
        invertColors = !invertColors;
        apiCall("/api/toggle-invert", "POST", null, function() { fetchStatus(); });
    }

    // Keyboard shortcuts
    Shortcut {
        sequence: "Escape"
        onActivated: {
            if (isStreaming) stopStream(); else window.close();
        }
    }

    Shortcut {
        sequence: "q"
        onActivated: {
            if (isStreaming) stopStream(); else window.close();
        }
    }

    Shortcut {
        sequence: "Space"
        onActivated: {
            if (isStreaming) stopStream(); else startStream();
        }
    }

    // Main layout
    ScrollView {
        anchors.fill: parent
        contentWidth: parent.width

        ColumnLayout {
            width: parent.width - 32
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: 14

            Item { height: 4 }

            // 1. Header & Theme Color Swatches Bar
            RowLayout {
                Layout.fillWidth: true
                spacing: 12

                Rectangle {
                    width: 44
                    height: 44
                    radius: 12
                    color: accentColor

                    Text {
                        anchors.centerIn: parent
                        text: "📷"
                        font.pixelSize: 22
                    }
                }

                ColumnLayout {
                    spacing: 2
                    Text {
                        text: "PhoneCam"
                        font.pixelSize: 20
                        font.bold: true
                        color: "#f4f4f5"
                    }
                    Text {
                        text: "Studio Smartphone Webcam for Omarchy"
                        font.pixelSize: 12
                        color: "#a1a1aa"
                    }
                }

                Item { Layout.fillWidth: true }

                // Refresh Button
                Button {
                    id: refreshBtn
                    objectName: "btn-refresh"
                    text: "🔄"
                    onClicked: {
                        fetchStatus();
                        fetchLogs();
                    }
                    contentItem: Text {
                        text: parent.text
                        color: "#e4e4e7"
                        font.pixelSize: 14
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                    }
                    background: Rectangle {
                        color: "#27272a"
                        radius: 8
                        border.color: "#3f3f46"
                        border.width: 1
                    }
                }
            }

            // 2. Device Status Card
            Rectangle {
                Layout.fillWidth: true
                height: 70
                radius: 12
                color: "#27272a"
                border.color: isDeviceConnected ? "#10b981" : "#ef4444"
                border.width: 1

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 12
                    spacing: 12

                    Rectangle {
                        width: 12
                        height: 12
                        radius: 6
                        color: isDeviceConnected ? "#10b981" : "#ef4444"
                    }

                    ColumnLayout {
                        spacing: 2
                        Text {
                            id: textDeviceName
                            objectName: "text-device-name"
                            text: deviceName
                            font.pixelSize: 15
                            font.bold: true
                            color: "#f4f4f5"
                        }
                        Text {
                            text: isDeviceConnected ? ("ADB Serial: " + deviceSerial) : "Connect phone via USB-C & Enable USB Debugging"
                            font.pixelSize: 12
                            color: "#a1a1aa"
                        }
                    }

                    Item { Layout.fillWidth: true }

                    Rectangle {
                        visible: batteryText.length > 0
                        width: 76
                        height: 28
                        radius: 6
                        color: "#3f3f46"

                        Text {
                            id: textBattery
                            objectName: "text-battery"
                            anchors.centerIn: parent
                            text: "🔋 " + batteryText
                            font.pixelSize: 12
                            font.bold: true
                            color: "#f4f4f5"
                        }
                    }
                }
            }

            // 3. Wireless ADB over Wi-Fi Card
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: wifiCol.implicitHeight + 28
                radius: 12
                color: "#27272a"
                border.color: "#3f3f46"
                border.width: 1

                ColumnLayout {
                    id: wifiCol
                    anchors.fill: parent
                    anchors.margins: 14
                    spacing: 10

                    RowLayout {
                        Layout.fillWidth: true
                        Text {
                            text: "WIRELESS ADB OVER WI-FI"
                            font.pixelSize: 11
                            font.bold: true
                            color: accentColor
                        }
                        Item { Layout.fillWidth: true }
                        Text {
                            text: "Untethered studio camera streaming"
                            font.pixelSize: 11
                            color: "#71717a"
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        TextField {
                            id: wifiAddressInput
                            objectName: "input-wifi-address"
                            Layout.fillWidth: true
                            Layout.preferredHeight: 38
                            placeholderText: "Phone IP:Port (e.g. 192.168.1.50:5555)"
                            placeholderTextColor: "#71717a"
                            color: "#f4f4f5"
                            font.pixelSize: 13
                            font.family: "Monospace"
                            selectByMouse: true
                            onTextChanged: wifiAddressText = text
                            onAccepted: connectWifi(text)
                            background: Rectangle {
                                color: "#18181b"
                                radius: 8
                                border.color: wifiAddressInput.activeFocus ? accentColor : "#3f3f46"
                                border.width: 1
                            }
                        }

                        Button {
                            id: btnWifiConnect
                            objectName: "btn-wifi-connect"
                            Layout.preferredHeight: 38
                            Layout.preferredWidth: 140
                            text: isWifiConnecting ? "⏳ Connecting..." : "📡 Connect Wi-Fi"
                            enabled: !isWifiConnecting
                            onClicked: connectWifi(wifiAddressInput.text)
                            contentItem: Text {
                                text: parent.text
                                font.pixelSize: 12
                                font.bold: true
                                color: "#18181b"
                                horizontalAlignment: Text.AlignHCenter
                                verticalAlignment: Text.AlignVCenter
                            }
                            background: Rectangle {
                                color: accentColor
                                radius: 8
                            }
                        }

                        Button {
                            id: btnWifiTcpip
                            objectName: "btn-wifi-tcpip"
                            Layout.preferredHeight: 38
                            Layout.preferredWidth: 175
                            text: "🔌 Enable USB TCP/IP"
                            enabled: !isWifiConnecting
                            onClicked: enableTcpip()
                            contentItem: Text {
                                text: parent.text
                                font.pixelSize: 12
                                font.bold: true
                                color: "#e4e4e7"
                                horizontalAlignment: Text.AlignHCenter
                                verticalAlignment: Text.AlignVCenter
                            }
                            background: Rectangle {
                                color: "#3f3f46"
                                radius: 8
                            }
                        }
                    }

                    Text {
                        id: textWifiStatus
                        objectName: "text-wifi-status"
                        Layout.fillWidth: true
                        text: wifiStatusMsg.length > 0 ? wifiStatusMsg : "💡 Initial setup: Connect phone via USB once and click 'Enable USB TCP/IP', then unplug cable and click 'Connect Wi-Fi'."
                        color: wifiStatusMsg.length > 0 ? (wifiStatusIsError ? "#ef4444" : "#10b981") : "#71717a"
                        font.pixelSize: 11
                        wrapMode: Text.Wrap
                    }
                }
            }

            // 4. Camera Selection Card (Bidirectionally Synchronized)
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: cameraCol.implicitHeight + 28
                radius: 12
                color: "#27272a"
                border.color: "#3f3f46"
                border.width: 1

                ColumnLayout {
                    id: cameraCol
                    anchors.fill: parent
                    anchors.margins: 14
                    spacing: 12

                    RowLayout {
                        Layout.fillWidth: true
                        Text {
                            text: "CAMERA SELECTION & SENSOR"
                            font.pixelSize: 11
                            font.bold: true
                            color: accentColor
                        }
                        Item { Layout.fillWidth: true }
                        Text {
                            text: "Auto-synced buttons & lens list"
                            font.pixelSize: 11
                            color: "#71717a"
                        }
                    }

                    // Facing Segmented Toggle
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        Button {
                            id: rearBtn
                            objectName: "btn-camera-rear"
                            Layout.fillWidth: true
                            height: 38
                            text: "📸 Rear Camera (Back)"
                            checkable: true
                            checked: currentFacing === "back"
                            onClicked: selectFacing("back")
                            contentItem: Text {
                                text: parent.text
                                font.pixelSize: 13
                                font.bold: parent.checked
                                color: parent.checked ? "#18181b" : "#e4e4e7"
                                horizontalAlignment: Text.AlignHCenter
                                verticalAlignment: Text.AlignVCenter
                            }
                            background: Rectangle {
                                color: parent.checked ? accentColor : "#3f3f46"
                                radius: 8
                            }
                        }

                        Button {
                            id: frontBtn
                            objectName: "btn-camera-front"
                            Layout.fillWidth: true
                            height: 38
                            text: "🤳 Selfie Camera (Front)"
                            checkable: true
                            checked: currentFacing === "front"
                            onClicked: selectFacing("front")
                            contentItem: Text {
                                text: parent.text
                                font.pixelSize: 13
                                font.bold: parent.checked
                                color: parent.checked ? "#18181b" : "#e4e4e7"
                                horizontalAlignment: Text.AlignHCenter
                                verticalAlignment: Text.AlignVCenter
                            }
                            background: Rectangle {
                                color: parent.checked ? accentColor : "#3f3f46"
                                radius: 8
                            }
                        }
                    }

                    // Sensor Dropdown (Syncs with Facing Buttons)
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        Text {
                            text: "Specific Lens:"
                            color: "#d4d4d8"
                            font.pixelSize: 13
                            Layout.preferredWidth: 100
                        }

                        ComboBox {
                            id: cameraCombo
                            objectName: "combo-camera-lens"
                            Layout.fillWidth: true
                            model: {
                                var items = [];
                                for (var i = 0; i < camerasList.length; i++) {
                                    var c = camerasList[i];
                                    items.push("Camera #" + c.id + " (" + c.facing.toUpperCase() + " — " + c.resolution + ", zoom " + c.zoom_range + ")");
                                }
                                return items.length > 0 ? items : ["Default Camera"];
                            }
                            onActivated: (index) => onCameraComboSelected(index)
                        }
                    }
                }
            }

            // 5. Geometry & Cut / Crop Card
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: geomCol.implicitHeight + 28
                radius: 12
                color: "#27272a"
                border.color: "#3f3f46"
                border.width: 1

                ColumnLayout {
                    id: geomCol
                    anchors.fill: parent
                    anchors.margins: 14
                    spacing: 12

                    Text {
                        text: "CUT (CROP) & RESIZE"
                        font.pixelSize: 11
                        font.bold: true
                        color: accentColor
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 12

                        // Resolution / Resize
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 4
                            Text {
                                text: "Resolution (Resize):"
                                color: "#d4d4d8"
                                font.pixelSize: 12
                            }
                            ComboBox {
                                id: resCombo
                                objectName: "combo-resolution"
                                Layout.fillWidth: true
                                model: ["1920x1080 (1080p FHD)", "1280x720 (720p HD)", "3840x2160 (4K UHD)", "2560x1440 (2K QHD)"]
                                currentIndex: currentRes === "1280x720" ? 1 : (currentRes === "3840x2160" ? 2 : 0)
                                onActivated: (index) => {
                                    if (index === 0) currentRes = "1920x1080";
                                    else if (index === 1) currentRes = "1280x720";
                                    else if (index === 2) currentRes = "3840x2160";
                                    else if (index === 3) currentRes = "2560x1440";
                                }
                            }
                        }

                        // Crop / Cut Preset
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 4
                            Text {
                                text: "Framing / Cut (Crop):"
                                color: "#d4d4d8"
                                font.pixelSize: 12
                            }
                            ComboBox {
                                id: cropCombo
                                objectName: "combo-crop"
                                Layout.fillWidth: true
                                model: [
                                    "Full Sensor (Native)",
                                    "16:9 Widescreen Crop",
                                    "4:3 Standard Crop",
                                    "1:1 Square (Avatar)",
                                    "Custom 1080p Center Cut"
                                ]
                                currentIndex: 0
                            }
                        }
                    }
                }
            }

            // 6. Transform & Image Invert / Flip Card
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: fxCol.implicitHeight + 28
                radius: 12
                color: "#27272a"
                border.color: "#3f3f46"
                border.width: 1

                ColumnLayout {
                    id: fxCol
                    anchors.fill: parent
                    anchors.margins: 14
                    spacing: 12

                    Text {
                        text: "TRANSFORM, INVERT & FILTERS"
                        font.pixelSize: 11
                        font.bold: true
                        color: accentColor
                    }

                    GridLayout {
                        columns: 2
                        rowSpacing: 10
                        columnSpacing: 16
                        Layout.fillWidth: true

                        // Horizontal Flip / Mirror
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8
                            Switch {
                                id: flipHSwitch
                                objectName: "switch-flip-h"
                                checked: flipH
                                onToggled: toggleFlipH()
                            }
                            ColumnLayout {
                                spacing: 1
                                Text { text: "Mirror (Flip Horizontal)"; color: "#f4f4f5"; font.pixelSize: 13 }
                                Text { text: "Natural webcam orientation"; color: "#a1a1aa"; font.pixelSize: 11 }
                            }
                        }

                        // Vertical Flip
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8
                            Switch {
                                id: flipVSwitch
                                objectName: "switch-flip-v"
                                checked: flipV
                                onToggled: toggleFlipV()
                            }
                            ColumnLayout {
                                spacing: 1
                                Text { text: "Flip Vertical (180°)"; color: "#f4f4f5"; font.pixelSize: 13 }
                                Text { text: "Inverted mount correction"; color: "#a1a1aa"; font.pixelSize: 11 }
                            }
                        }

                        // Color Invert
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8
                            Switch {
                                id: invertSwitch
                                objectName: "switch-invert"
                                checked: invertColors
                                onToggled: toggleInvert()
                            }
                            ColumnLayout {
                                spacing: 1
                                Text { text: "Invert Colors (Negative)"; color: "#f4f4f5"; font.pixelSize: 13 }
                                Text { text: "Hardware ffmpeg negation"; color: "#a1a1aa"; font.pixelSize: 11 }
                            }
                        }

                        // Torch
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8
                            Switch {
                                id: torchSwitch
                                objectName: "switch-torch"
                                checked: torchOn
                                enabled: currentFacing === "back"
                                onToggled: { torchOn = !torchOn; }
                            }
                            ColumnLayout {
                                spacing: 1
                                Text { text: "Camera Torch (Flashlight)"; color: "#f4f4f5"; font.pixelSize: 13 }
                                Text { text: "Rear lens studio lighting"; color: "#a1a1aa"; font.pixelSize: 11 }
                            }
                        }

                        // Preview Window Toggle
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8
                            Switch {
                                id: previewSwitch
                                objectName: "switch-preview"
                                checked: showPreview
                                onToggled: { showPreview = !showPreview; }
                            }
                            ColumnLayout {
                                spacing: 1
                                Text { text: "Live Desktop Preview"; color: "#f4f4f5"; font.pixelSize: 13 }
                                Text { text: "Open real-time monitor window"; color: "#a1a1aa"; font.pixelSize: 11 }
                            }
                        }
                    }
                }
            }

            // 7. Virtual Webcam Device & Stream Action Card
            Rectangle {
                Layout.fillWidth: true
                height: 96
                radius: 12
                color: "#27272a"
                border.color: isStreaming ? "#10b981" : "#3f3f46"
                border.width: 2

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 14
                    spacing: 16

                    ColumnLayout {
                        spacing: 2
                        Text {
                            id: textStreamStatus
                            objectName: "text-stream-status"
                            text: isStreaming ? "● VIRTUAL CAMERA ACTIVE" : "VIRTUAL CAMERA OFFLINE"
                            font.pixelSize: 14
                            font.bold: true
                            color: isStreaming ? "#10b981" : "#a1a1aa"
                        }
                        Text {
                            text: "Device node: /dev/video10 (v4l2loopback)"
                            font.pixelSize: 12
                            color: "#71717a"
                        }
                    }

                    Item { Layout.fillWidth: true }

                    // Dedicated Preview Button
                    Button {
                        id: previewBtn
                        objectName: "btn-preview"
                        Layout.preferredWidth: 130
                        Layout.preferredHeight: 46
                        text: "👁️ Preview"
                        onClicked: {
                            showPreview = true;
                            startStream();
                        }
                        contentItem: Text {
                            text: parent.text
                            font.pixelSize: 13
                            font.bold: true
                            color: "#e4e4e7"
                            horizontalAlignment: Text.AlignHCenter
                            verticalAlignment: Text.AlignVCenter
                        }
                        background: Rectangle {
                            color: "#3f3f46"
                            radius: 10
                        }
                    }

                    // Main Action Button
                    Button {
                        id: startBtn
                        objectName: "btn-stream-action"
                        Layout.preferredWidth: 170
                        Layout.preferredHeight: 46
                        text: isStreaming ? "🛑 Stop & Detach" : "🚀 Start Webcam"
                        onClicked: {
                            if (isStreaming) {
                                stopStream();
                            } else {
                                startStream();
                            }
                        }
                        contentItem: Text {
                            text: parent.text
                            font.pixelSize: 14
                            font.bold: true
                            color: isStreaming ? "#ffffff" : "#18181b"
                            horizontalAlignment: Text.AlignHCenter
                            verticalAlignment: Text.AlignVCenter
                        }
                        background: Rectangle {
                            color: isStreaming ? "#ef4444" : accentColor
                            radius: 10
                        }
                    }
                }
            }

            // 8. Live Console & Diagnostics
            Rectangle {
                Layout.fillWidth: true
                height: 130
                radius: 12
                color: "#1c1917"
                border.color: "#292524"
                border.width: 1

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 10
                    spacing: 4

                    RowLayout {
                        Layout.fillWidth: true
                        Text {
                            text: "LOGS & DIAGNOSTICS"
                            font.pixelSize: 11
                            font.bold: true
                            color: "#78716c"
                        }
                        Item { Layout.fillWidth: true }
                        Text {
                            text: "Press ESC or 'q' to stop & detach"
                            font.pixelSize: 11
                            color: "#57534e"
                        }
                    }

                    ListView {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        clip: true
                        model: recentLogs
                        delegate: Text {
                            text: modelData
                            font.family: "Monospace"
                            font.pixelSize: 11
                            color: "#a8a29e"
                        }
                    }
                }
            }

            Item { height: 16 }
        }
    }
}
