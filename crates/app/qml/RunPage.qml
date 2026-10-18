import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Kirigami.Page {
    id: page

    required property var controller
    readonly property bool running: controller.phase === "running"
    readonly property bool succeeded: controller.phase === "done"

    // Escape, Alt+Left, the mouse back button and swipes must not leave the page mid-write.
    onBackRequested: event => {
        if (running) {
            event.accepted = true
        }
    }

    title: running ? "Formatage en cours" : (succeeded ? "Terminé" : "Échec")
    // No back button while the disk is being written.
    globalToolBarStyle: running ? Kirigami.ApplicationHeaderStyle.None : Kirigami.ApplicationHeaderStyle.Auto

    ColumnLayout {
        anchors.centerIn: parent
        width: Math.min(parent.width - Kirigami.Units.gridUnit * 4, Kirigami.Units.gridUnit * 30)
        spacing: Kirigami.Units.largeSpacing

        Kirigami.Icon {
            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: Kirigami.Units.iconSizes.huge
            Layout.preferredHeight: Kirigami.Units.iconSizes.huge
            visible: !page.running
            source: page.succeeded ? "emblem-success" : "dialog-error"
        }

        QQC2.Label {
            Layout.fillWidth: true
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.Wrap
            font.bold: true
            text: page.running ? page.controller.stepText : page.controller.resultMessage
        }

        QQC2.ProgressBar {
            Layout.fillWidth: true
            visible: page.running
            // Steps are few and uneven, so keep the bar moving with the current step.
            indeterminate: page.controller.progress === 0
            value: page.controller.progress
        }

        QQC2.Label {
            Layout.fillWidth: true
            visible: page.running
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.Wrap
            opacity: 0.7
            text: "Ne débranchez pas le disque et ne fermez pas l'application."
        }

        QQC2.Button {
            Layout.alignment: Qt.AlignHCenter
            visible: !page.running
            text: page.succeeded ? "Terminer" : "Retour à la liste"
            icon.name: "go-home"
            onClicked: {
                applicationWindow().pageStack.pop(applicationWindow().pageStack.get(0))
                page.controller.reset()
                page.controller.refresh()
            }
        }
    }
}
