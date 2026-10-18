import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Kirigami.ScrollablePage {
    id: page

    required property var controller
    readonly property var plan: JSON.parse(controller.planJson)
    property string typed: ""
    readonly property bool confirmed: typed === plan.confirmationPhrase

    title: "Confirmer le formatage"

    header: Kirigami.InlineMessage {
        visible: controller.planError !== ""
        type: Kirigami.MessageType.Error
        text: controller.planError
    }

    // Move on to the progress screen as soon as the controller starts writing.
    Connections {
        target: page.controller
        function onPhaseChanged() {
            if (page.controller.phase === "running" && applicationWindow().pageStack.currentItem === page) {
                // Replace (not push) so "back" can never return to a confirmation that already ran.
                applicationWindow().pageStack.replace(Qt.resolvedUrl("RunPage.qml"), { controller: page.controller })
            }
        }
    }

    footer: QQC2.ToolBar {
        RowLayout {
            anchors.fill: parent
            QQC2.TextField {
                id: phraseField
                Layout.fillWidth: true
                placeholderText: "Tapez « " + page.plan.confirmationPhrase + " » pour confirmer"
                onTextEdited: page.typed = text
                onAccepted: if (page.confirmed) page.controller.confirmAndRun(page.typed)
            }
            QQC2.Button {
                text: "Formater définitivement"
                icon.name: "edit-delete"
                enabled: page.confirmed
                onClicked: page.controller.confirmAndRun(page.typed)
            }
        }
    }

    ColumnLayout {
        spacing: Kirigami.Units.largeSpacing

        Kirigami.InlineMessage {
            Layout.fillWidth: true
            visible: true
            type: Kirigami.MessageType.Warning
            text: "Toutes les données de ce disque seront définitivement effacées. Cette action est irréversible."
        }

        Kirigami.Heading {
            Layout.fillWidth: true
            level: 2
            wrapMode: Text.Wrap
            text: page.plan.title + " — " + page.plan.sizeText + " (" + page.plan.device + ")"
        }

        Kirigami.Heading {
            level: 4
            text: "Ce qui sera effacé"
        }
        QQC2.Label {
            visible: page.plan.willErase.length === 0
            opacity: 0.7
            text: "Aucune partition détectée sur ce disque."
        }
        Repeater {
            model: page.plan.willErase
            delegate: QQC2.Label {
                required property var modelData
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                text: "• " + modelData.device + " — "
                    + (modelData.label !== "" ? "« " + modelData.label + " », " : "")
                    + (modelData.fsType !== "" ? modelData.fsType + ", " : "")
                    + modelData.sizeText
                    + (modelData.mounted ? " (sera démontée)" : "")
            }
        }

        Kirigami.Heading {
            level: 4
            text: "Nouveau format"
        }
        QQC2.Label {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            text: page.plan.fsName + " · table " + page.plan.tableName
                + (page.plan.label !== "" ? " · nom « " + page.plan.label + " »" : " · sans nom")
        }
        QQC2.Label {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            opacity: 0.7
            text: page.plan.eraseText
        }

        Kirigami.Heading {
            level: 4
            text: "Étapes"
        }
        Repeater {
            model: page.plan.steps
            delegate: QQC2.Label {
                required property string modelData
                required property int index
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                text: (index + 1) + ". " + modelData
            }
        }
    }
}
