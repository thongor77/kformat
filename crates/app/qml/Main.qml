import QtQuick
import org.kde.kirigami as Kirigami
import org.kformat.app

Kirigami.ApplicationWindow {
    id: root

    title: "kformat"
    width: 720
    height: 600
    visible: true

    DiskController { id: controller }

    // Message about the `--device` launch option (device missing, system disk, bad value).
    property string startNotice: controller.startError
    property bool startHandled: false

    pageStack.initialPage: DiskPage {
        controller: controller
        notice: root.startNotice
    }

    // Launched with `--device /dev/sdX` (e.g. from the device notifier): skip the disk list.
    function openStartDevice() {
        if (startHandled || controller.startDevice === "") {
            return
        }
        startHandled = true
        const wanted = controller.startDevice
        const disk = JSON.parse(controller.disksJson).find(
            d => d.device === wanted || d.partitions.some(p => p.device === wanted)
        )
        if (!disk) {
            startNotice = "Disque introuvable : " + wanted
        } else if (!disk.selectable) {
            startNotice = disk.title + " : " + disk.blockedReason
        } else {
            pageStack.push(Qt.resolvedUrl("OptionsPage.qml"), { controller: controller, disk: disk })
        }
    }

    Connections {
        target: controller
        function onBusyChanged() {
            if (!controller.busy && controller.error === "") {
                root.openStartDevice()
            }
        }
    }

    Component.onCompleted: {
        controller.loadOptions()
        controller.refresh()
    }

    // Never let the window close while a disk is being written.
    onClosing: close => {
        if (controller.phase === "running") {
            close.accepted = false
        }
    }

    // Only present in builds made with the `smoke` feature.
    Loader {
        active: controller.smokeEnabled() && Qt.platform.pluginName === "offscreen"
        source: Qt.resolvedUrl("SmokeTest.qml")
        onLoaded: item.controller = controller
    }
}
