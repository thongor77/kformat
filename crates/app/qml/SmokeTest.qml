import QtQuick
import org.kde.kirigami as Kirigami

// Headless end-to-end check, only loaded with the offscreen platform.
// Without `--smoke <device>` it just reports the disk list.
// With it, formats that (loop) device through the same controller calls the UI uses,
// first with a wrong confirmation, then with the right one.
Item {
    id: smoke

    property var controller
    readonly property var args: Qt.application.arguments
    readonly property int flag: args.indexOf("--smoke")
    readonly property string device: flag >= 0 ? args[flag + 1] : ""
    property int tick: 0

    function log(...parts) { console.log("smoke:", ...parts) }

    Timer {
        interval: 700
        repeat: true
        running: smoke.controller !== undefined
        onTriggered: {
            smoke.tick++
            const c = smoke.controller
            if (smoke.device !== "" && !smoke.device.startsWith("/dev/loop")) {
                smoke.log("refusing non-loop device:", smoke.device)
                Qt.quit()
                return
            }
            if (smoke.device === "") {
                if (smoke.tick === 4) {
                    smoke.log("disks:", JSON.parse(c.disksJson).length, "error:", c.error)
                    smoke.log("start:", JSON.stringify(c.startDevice), "startError:", JSON.stringify(c.startError),
                              "| page:", applicationWindow().pageStack.currentItem.title)
                    smoke.log("lang check:", c.tr("DiskTitle"), "|", c.tr1("StartNotFound", "/dev/sdx"),
                              "|", JSON.parse(c.optionsJson).presets[0].title)
                    Qt.quit()
                }
                return
            }
            switch (smoke.tick) {
            case 1:
                c.preparePlan(smoke.device, "ext4", "gpt", "SMOKE", "quick")
                break
            case 3:
                smoke.log("phase after prepare:", c.phase, "| planError:", c.planError)
                smoke.log("plan:", c.planJson)
                c.confirmAndRun("not-the-disk")
                smoke.log("wrong confirmation -> phase:", c.phase, "| error:", c.planError)
                break
            case 4:
                c.confirmAndRun(JSON.parse(c.planJson).confirmationPhrase)
                break
            case 8:
                smoke.log("final phase:", c.phase, "| progress:", c.progress, "|", c.resultMessage)
                Qt.quit()
                break
            }
        }
    }
}
