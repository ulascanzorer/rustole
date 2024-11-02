import QtQuick 2.12
import QtQuick.Controls 2.12
import QtQuick.Window 2.12

// This must match the uri and version
// specified in the qml_module in the build.rs script.
import com.kdab.cxx_qt.demo 1.0

ApplicationWindow {
    height: 480
    title: qsTr("rustole")
    visible: true
    width: 640
    color: "#000000"

    TerminalTextObject {
        id: terminalTextObject
        inputBuffer: ""
        outputBuffer: ""
        inputStartIndex: 0
    }

    Flickable {
        id: flick
        width: parent.width
        height: parent.height
        contentWidth: mainTextArea.contentWidth
        contentHeight: mainTextArea.contentHeight
        clip: true
        flickableDirection: Flickable.VerticalFlick // Only allow vertical flicking.
        
        function ensureVisible(r) {
            if (contentX >= r.x)
                contentX = r.x;
            else if (contentX+width <= r.x+r.width)
                contentX = r.x+r.width-width;
            if (contentY >= r.y)
                contentY = r.y;
            else if (contentY+height <= r.y+r.height)
                contentY = r.y+r.height-height;
        }
        
        TextEdit {
            id: mainTextArea
            width: flick.width
            wrapMode: Text.Wrap
            focus: true
            color: "white"
            text: ""
            font.pointSize: 20

            
            // Logic for key presses.
            Keys.onPressed: {
                // Stops the user from being able to edit the lines that came before.
                if (mainTextArea.cursorPosition < terminalTextObject.inputStartIndex) {
                    mainTextArea.cursorPosition = terminalTextObject.inputStartIndex;
                } else if (mainTextArea.cursorPosition == terminalTextObject.inputStartIndex && (event.key === Qt.Key_Backspace || event.key === Qt.Key_Left || event.key === Qt.Key_Up)) {
                    // console.log("I am here!");
                    event.accepted = true;
                    return;
                }

                // Disable selecting everything, so that users can't cheat their way to delete previous lines.
                if (event.key === Qt.Key_A && (event.modifiers & Qt.ControlModifier)) {
                    event.accepted = true; // Block the default "select all" behavior
                    return;
                }


                // Special logic for the space key so that we wrap correctly.
                if (event.key === Qt.Key_Space) {
                    if (mainTextArea.cursorRectangle.x >= mainTextArea.width - 10) {
                        terminalTextObject.appendInputBuffer("\n");
                        // terminalTextObject.appendTerminalBuffer("\n");
                    } else {
                        terminalTextObject.appendInputBuffer(" ");
                        // terminalTextObject.appendTerminalBuffer(" ");
                    }
                    // event.accepted = true;
                }

                // Send the written command to our main processing function when the "Return" (Enter) key is pressed.
                else if (event.key === Qt.Key_Return) {
                    let userCommand = mainTextArea.text.substring(terminalTextObject.inputStartIndex);
                    terminalTextObject.appendInputBuffer(userCommand);
                    terminalTextObject.processCommand();
                    // event.accepted = true; // If this is uncommented, event will be ignored so in this case we won't go to the next line.
                }
            }

            // Allows us to scroll.
            onCursorRectangleChanged: flick.ensureVisible(cursorRectangle)
        }

        // Signal handler to update the GUI with the outputs of commands.

        Connections {
            target: terminalTextObject

            function onOutputBufferUpdated() {
                mainTextArea.insert(mainTextArea.length, terminalTextObject.outputBuffer);
                // console.log("This is the input start index: ", terminalTextObject.inputStartIndex);
            }
        }
    }
}
