import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ApplicationWindow {
    id: window
    visible: true
    width: 680
    height: 760
    minimumWidth: 540
    minimumHeight: 650
    title: "PhoneCam — Smartphone Virtual Webcam"
    color: "#18181b"

    // Runtime state
    property int port: 41789
    property string accentColor: "#FFD60A"
    property bool isStreaming: false
    property bool isDeviceConnected: false
    property string deviceName: "Detecting..."
    property string deviceSerial: ""
    property string batteryText: ""
    property var camerasList: []
    property var recentLogs: []

    // Configuration model
    property string currentFacing: "back"
    property int currentCameraId: 0
    property string currentRes: "1920x1080"
    property string currentCrop: ""
    property int currentFps: 30
    property bool flipH: false
    property bool flipV: false
    property bool invertColors: false
    property bool showPreview: false
    property bool torchOn: false

    Component.onCompleted: {
        var envPort = Qt.application.arguments;
        // Check environment variable passed
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
                if (xhr.status === 200) {
                    try {
                        var res = JSON.parse(xhr.responseText);
                        if (callback) callback(res);
                    } catch (e) {
                        console.log("Parse error: " + e);
                    }
                }
            }
        };
        xhr.send(payload ? JSON.stringify(payload) : null);
    }

    function fetchStatus() {
        apiCall("/api/status", "GET", null, function(data) {
            if (data.accent_color) {
                accentColor = data.accent_color;
            }
            isStreaming = data.is_streaming;
            if (data.device) {
                if (data.device.status === "Connected") {
                    isDeviceConnected = true;
                    deviceName = data.device.brand.toUpperCase() + " " + data.device.model;
                    deviceSerial = data.device.serial;
                    if (data.device.battery_percent !== null) {
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
            if (data.cameras) {
                camerasList = data.cameras;
            }
            if (data.config) {
                currentFacing = data.config.facing;
                if (data.config.camera_id !== null && data.config.camera_id !== undefined) {
                    currentCameraId = data.config.camera_id;
                }
                currentRes = data.config.resolution;
                currentFps = data.config.fps;
                flipH = data.config.flip_h;
                flipV = data.config.flip_v;
                invertColors = data.config.invert;
                torchOn = data.config.torch;
                showPreview = data.config.preview;
            }
        });
    }

    function fetchLogs() {
        apiCall("/api/logs", "GET", null, function(data) {
            recentLogs = data;
        });
    }

    function startStream() {
        var cropVal = null;
        if (cropCombo.currentIndex === 1) { // 16:9
            cropVal = "1920:1080:0:0";
        } else if (cropCombo.currentIndex === 2) { // 4:3
            cropVal = "1440:1080:240:0";
        } else if (cropCombo.currentIndex === 3) { // 1:1 Square
            cropVal = "1080:1080:420:0";
        }

        var cfg = {
            video_nr: 10,
            card_label: "PhoneCam Virtual Camera",
            facing: currentFacing,
            camera_id: cameraCombo.currentIndex >= 0 && cameraCombo.currentIndex < camerasList.length ? camerasList[cameraCombo.currentIndex].id : null,
            resolution: currentRes,
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
            if (isStreaming) {
                stopStream();
            } else {
                window.close();
            }
        }
    }

    Shortcut {
        sequence: "q"
        onActivated: {
            if (isStreaming) {
                stopStream();
            } else {
                window.close();
            }
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
            spacing: 16

            Item { height: 8 }

            // 1. Header & Title Bar
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
                        text: "Virtual Webcam for Omarchy & Linux"
                        font.pixelSize: 12
                        color: "#a1a1aa"
                    }
                }

                Item { Layout.fillWidth: true }

                // Refresh Button
                Button {
                    text: "🔄 Refresh"
                    onClicked: {
                        fetchStatus();
                        fetchLogs();
                    }
                    contentItem: Text {
                        text: parent.text
                        color: "#e4e4e7"
                        font.pixelSize: 12
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
                height: 72
                radius: 12
                color: "#27272a"
                border.color: isDeviceConnected ? "#10b981" : "#ef4444"
                border.width: 1

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 14
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
                            text: deviceName
                            font.pixelSize: 15
                            font.bold: true
                            color: "#f4f4f5"
                        }
                        Text {
                            text: isDeviceConnected ? ("ADB Serial: " + deviceSerial) : "Plug in USB-C & Enable USB Debugging"
                            font.pixelSize: 12
                            color: "#a1a1aa"
                        }
                    }

                    Item { Layout.fillWidth: true }

                    // Battery Badge
                    Rectangle {
                        visible: batteryText.length > 0
                        width: 72
                        height: 28
                        radius: 6
                        color: "#3f3f46"

                        Text {
                            anchors.centerIn: parent
                            text: "🔋 " + batteryText
                            font.pixelSize: 12
                            font.bold: true
                            color: "#f4f4f5"
                        }
                    }
                }
            }

            // 3. Camera Selection Card
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

                    Text {
                        text: "CAMERA SELECTION"
                        font.pixelSize: 11
                        font.bold: true
                        color: accentColor
                    }

                    // Facing Segmented Toggle
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        Button {
                            Layout.fillWidth: true
                            height: 38
                            text: "📸 Rear Camera (Back)"
                            checkable: true
                            checked: currentFacing === "back"
                            onClicked: {
                                currentFacing = "back";
                                apiCall("/api/set-facing", "POST", { facing: "back" }, function() { fetchStatus(); });
                            }
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
                            Layout.fillWidth: true
                            height: 38
                            text: "🤳 Selfie Camera (Front)"
                            checkable: true
                            checked: currentFacing === "front"
                            onClicked: {
                                currentFacing = "front";
                                apiCall("/api/set-facing", "POST", { facing: "front" }, function() { fetchStatus(); });
                            }
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

                    // Specific Sensor Dropdown
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        Text {
                            text: "Sensor / Lens:"
                            color: "#d4d4d8"
                            font.pixelSize: 13
                            Layout.preferredWidth: 100
                        }

                        ComboBox {
                            id: cameraCombo
                            Layout.fillWidth: true
                            model: {
                                var items = [];
                                for (var i = 0; i < camerasList.length; i++) {
                                    var c = camerasList[i];
                                    items.push("Camera #" + c.id + " (" + c.facing + " — " + c.resolution + ", zoom " + c.zoom_range + ")");
                                }
                                return items.length > 0 ? items : ["Default Camera"];
                            }
                            onActivated: {
                                if (index >= 0 && index < camerasList.length) {
                                    var selected = camerasList[index];
                                    currentFacing = selected.facing;
                                    currentCameraId = selected.id;
                                    apiCall("/api/select-camera", "POST", { id: selected.id, facing: selected.facing }, function() { fetchStatus(); });
                                }
                            }
                        }
                    }
                }
            }

            // 4. Geometry & Cut / Resize Card
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
                                Layout.fillWidth: true
                                model: ["1920x1080 (1080p FHD)", "1280x720 (720p HD)", "3840x2160 (4K UHD)", "2560x1440 (2K QHD)"]
                                currentIndex: currentRes === "1280x720" ? 1 : (currentRes === "3840x2160" ? 2 : 0)
                                onActivated: {
                                    if (index === 0) currentRes = "1920x1080";
                                    else if (index === 1) currentRes = "1280x720";
                                    else if (index === 2) currentRes = "3840x2160";
                                    else if (index === 3) currentRes = "2560x1440";
                                }
                            }
                        }

                        // Crop / Cut
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
                                Layout.fillWidth: true
                                model: ["Full Sensor (None)", "16:9 Widescreen Crop", "4:3 Standard Crop", "1:1 Square (Avatar)"]
                                currentIndex: 0
                            }
                        }
                    }
                }
            }

            // 5. Transform & Image Invert / Flip Card
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
                                checked: flipV
                                onToggled: toggleFlipV()
                            }
                            ColumnLayout {
                                spacing: 1
                                Text { text: "Flip Vertical (180°)"; color: "#f4f4f5"; font.pixelSize: 13 }
                                Text { text: "Upside down mount fix"; color: "#a1a1aa"; font.pixelSize: 11 }
                            }
                        }

                        // Color Invert
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8
                            Switch {
                                id: invertSwitch
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
                                checked: torchOn
                                enabled: currentFacing === "back"
                                onToggled: {
                                    torchOn = !torchOn;
                                }
                            }
                            ColumnLayout {
                                spacing: 1
                                Text { text: "Camera Torch (Flashlight)"; color: "#f4f4f5"; font.pixelSize: 13 }
                                Text { text: "Illumination for rear lens"; color: "#a1a1aa"; font.pixelSize: 11 }
                            }
                        }
                    }
                }
            }

            // 6. Virtual Webcam Device & Stream Action Card
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

                    Button {
                        Layout.preferredWidth: 200
                        Layout.preferredHeight: 48
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
                            font.pixelSize: 15
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

            // 7. Live Console & Diagnostics
            Rectangle {
                Layout.fillWidth: true
                height: 140
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
