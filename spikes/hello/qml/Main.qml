import QtQuick
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import org.kformat.spike

Kirigami.ApplicationWindow {
    id: root
    width: 640
    height: 420
    visible: true
    title: "kformat spike"

    DiskModel { id: disks }

    pageStack.initialPage: Kirigami.Page {
        title: "Drives (udisks2)"
        QQC2.Label { anchors.fill: parent; text: disks.summary; wrapMode: Text.Wrap }
        actions: [ Kirigami.Action { text: "Refresh"; onTriggered: disks.refresh() } ]
    }

    Component.onCompleted: disks.refresh()
    Timer { interval: 1500; running: Qt.platform.pluginName === "offscreen"; onTriggered: Qt.quit() }
}
