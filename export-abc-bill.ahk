#NoEnv  ; Recommended for performance and compatibility with future AutoHotkey releases.
; #Warn  ; Enable warnings to assist with detecting common errors.
SendMode Input  ; Recommended for new scripts due to its superior speed and reliability.
SetWorkingDir %A_ScriptDir%  ; Ensures a consistent starting directory.

; Make a system call to generate a GUID. The returned GUID will be enclosed in
; curly braces "{}"
; If the system call fails, then return a null string
CreateGUID()
{
    VarSetCapacity(pguid, 16, 0)
    if !(DllCall("ole32.dll\CoCreateGuid", "ptr", &pguid)) {
        size := VarSetCapacity(sguid, (38 << !!A_IsUnicode) + 1, 0)
        if (DllCall("ole32.dll\StringFromGUID2", "ptr", &pguid, "ptr", &sguid, "int", size))
            return StrGet(&sguid)
    }
    return ""
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

GetPrices(SkusFileLocation) {
    FileRead, SkuStrs, %SkusFileLocation%

    if WinExist("REIFSNYDER'S AG CENTER - ABC Accounting Client")
        WinActivate
    else 
        Run, "C:\ABC Software\Client4\abctwin.exe"

    ; Go to inventory screen
    Send, {F10}
    Sleep, 1000
    Send, i
    Sleep, 1000

    PricesBySku := []
    Loop, parse, SkuStrs, `n 
    {
        TrimmedSku := Trim(A_LoopField)
        SplitSku := StrSplit(TrimmedSku, " ",, 2)

        ControlClick, ThunderRT6TextBox2
        if SplitSku.Length() == 2 {
            Send, {F6}
            Sleep, 1000
            Send, %TrimmedSku%
            Sleep, 500
            Send, {Enter}
            Sleep, 1000
        } else {
            TempSku := SplitSku[1]
            ControlSetText, ThunderRT6TextBox2, %TempSku%
            Send, {Enter}
            Sleep, 500
        }
        ControlGetText, ListPrice, ThunderRT6TextBox27
        ControlGetText, ActualSku, ThunderRT6TextBox2
        PricesBySku[ActualSku] := ListPrice
        MsgBox % ActualSku " " PricesBySku[ActualSku]
    }
}

Run214:
if WinExist("REIFSNYDER'S AG CENTER - ABC Accounting Client")
    WinActivate
else 
    Run, "C:\ABC Software\Client4\abctwin.exe"

; Get to the main menu of ABC and run the 2-14 report
Send, {F10}
Sleep, 1000
Send, 2
Sleep, 1000
Send, 14
Sleep, 500
Send, {Enter}

; On the report screen, send the starting and ending bill numbers 
Sleep, 1000
Send, {Enter}
Sleep, 500
Send, %StartingBill%
Sleep, 500
Send, {Enter}
Sleep, 500
Send, %EndingBill%

; Tell ABC to export the report to a text file
Sleep, 500
Send, {Enter}
Sleep, 500
Send, x
Sleep, 1000

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
BillFileLocation = %A_ScriptDir%\2_14_%FileID%.txt
SkusFileLocation = %A_ScriptDir%\skus_%FileID%.txt
Send, %BillFileLocation%
Sleep, 500
Send, {Enter}

; Match titles that contain the given string anywhere
SetTitleMatchMode, 2
loop, 300 {
    if (FileExist(BillFileLocation)) and (WinExist("2_14_" . FileID)) {
        Run, %A_ScriptDir%\.virtual\Scripts\python.exe %A_ScriptDir%\sku_extractor.py -o %SkusFileLocation% %BillFileLocation%

        loop, 300 {
            if (FileExist(SkusFileLocation)) {
                GetPrices(SkusFileLocation)
                break
            } else 
                Sleep, 1000
        }
        break
    }
    else
        Sleep, 1000
}
ExitApp

GoSub, ShowForm
