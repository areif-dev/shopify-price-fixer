#NoEnv  ; Recommended for performance and compatibility with future AutoHotkey releases.
; #Warn  ; Enable warnings to assist with detecting common errors.
SendMode Input  ; Recommended for new scripts due to its superior speed and reliability.
SetWorkingDir %A_ScriptDir%  ; Ensures a consistent starting directory.

#Include lib.ahk 

EnvGet, HomeDrive, homedrive 
EnvGet, HomePath, homepath
TabbedFile = %HomeDrive%%HomePath%\Documents\My ABC Files\TabOutput.tsv
ScreenDir = %A_ScriptDir%\screens\
ShortWait = 100
if FileExist(TabbedFile) {
    FileDelete % TabbedFile
}

; Subroutine to generate a form to enter starting and ending bill ids
ShowForm:
Gui, New,, Shopify Price Fixer 
Gui, Add, Text,, Starting Bill ID
Gui, Add, Edit, vStartingBill
Gui, Add, Text,, Ending Bill ID 
Gui, Add, Edit, vEndingBill
Gui, Add, Button, Default gVerifyInput, Submit
Gui, Show
Return

; Ensure that the starting and ending bill ids are valid input, and reprompt if necessary
VerifyInput:
Gui, Submit
Gui, Destroy
if StartingBill is not integer
{
    MsgBox, You must enter an integer for Starting Bill! 
    GoSub, ShowForm 
    Return
}
if (EndingBill = "") 
{
    EndingBill := StartingBill
}
if EndingBill is not integer 

{
    MsgBox, You must enter an integer for Ending Invoice!
    GoSub, ShowForm
}
GoSub, Run214
Return

Run214:
if WinExist("REIFSNYDER'S AG CENTER - ABC Accounting Client")
    WinActivate
else 
    Run, "C:\ABC Software\Client4\abctwin.exe"

; Get to the main menu of ABC and run the 2-14 report
Send, {F10}
AwaitElementLoad(ScreenDir . "selection_screen.png")
Send, 2
AwaitElementLoad(ScreenDir . "purchase_reports_screen.png")
Send, 14
Send, {Enter}

; On the report screen, send the starting and ending bill numbers 
AwaitElementLoad(ScreenDir . "2_14_screen.png")
Send, {Enter}
Sleep % ShortWait * 2
Send, %StartingBill%
Sleep % ShortWait * 2
Send, {Enter}
Sleep % ShortWait * 2
Send, %EndingBill%

; Tell ABC to export the report to a text file
Sleep % ShortWait * 2
Send, {Enter}
Sleep % ShortWait * 2
Send, t
Sleep % ShortWait * 10

FileGUID := CreateGUID()
if (StrLen(FileGUID) > 0) {
    ; The GUID creation function was successful, so make the file id the GUID
    ; GUID is preferred over time because it is guaranteed to be unique
    FileID := SubStr(FileGUID, 2, StrLen(FileGUID) - 2)
} else {
    ; The GUID creation failed, so use the current time as the file id
    ; Current time is very likely to be unique, but may not be if the script
    ; is run in very quick succession or the system time is reset
    FormatTime, FileID,, yyyy-MM-ddTHH_mm
}
SkusFileLocation = %A_ScriptDir%\skus_%FileID%.txt

loop, 300 {
    if (FileExist(TabbedFile)) {
        RunWait, "%A_ScriptDir%\.virtual\Scripts\python.exe" "%A_ScriptDir%\sku_extractor.py" -o "%SkusFileLocation%" "%TabbedFile%"

        loop, 300 {
            if (FileExist(SkusFileLocation)) {
                RunWait, C:\Program Files\AutoHotKey\AutoHotKey.exe %A_ScriptDir%\get-prices-by-sku.ahk %SkusFileLocation%
                break
            } else 
                Sleep % ShortWait * 10
        }
        break
    }
    else
        Sleep % ShortWait * 10
}
ExitApp

GoSub, ShowForm
