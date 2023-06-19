#NoEnv  ; Recommended for performance and compatibility with future AutoHotkey releases.
; #Warn  ; Enable warnings to assist with detecting common errors.
SendMode Input  ; Recommended for new scripts due to its superior speed and reliability.
SetWorkingDir %A_ScriptDir%  ; Ensures a consistent starting directory.

GetPrices(SkusFileLocation) {
    FileRead, SkuStrs, %SkusFileLocation%

    if WinExist("REIFSNYDER'S AG CENTER - ABC Accounting Client")
        WinActivate
    else 
        Run, "C:\ABC Software\Client4\abctwin.exe"

    ; Go to accounts receivable screen
    Send, {F10}
    Sleep, 1000
    Send, r
    Sleep, 1000

    ; Clear the screen to a new ar file
    Send, {Ctrl Down}
    Send, n 
    Send, {Ctrl Up}
    Sleep, 1000

    if WinExist("Save changes before proceeding?") {
        Send, {Right}
        Sleep, 100
        Send, {Enter}
        Sleep, 1000

        if WinExist("REIFSNYDER'S AG CENTER - ABC Accounting Client")
            WinActivate
    }

    Send, {PgDn}
    Sleep, 500

    PricesBySku := []
    Loop, parse, SkuStrs, `n 
    {
        TrimmedSku := Trim(A_LoopField, OmitChars := "`n`t`r")

        Send, %TrimmedSku%
        Send, {Enter}
        Sleep, 500
        WinGetText, WinText
        SplitWinText := StrSplit(WinText, " ")

        if (SplitWinText[1] = "Lookup") {
            Send, %TrimmedSku%
            Sleep, 1000
            Send, {Enter}
            Sleep, 500
        }
        
        Sleep, 500
        Send, {Up}
        Sleep, 500

        ControlGetText, ListPrice, ThunderRT6TextBox45 
        ControlGetText, ActualSku, ThunderRT6TextBox41
        PricesBySku[ActualSku] := ListPrice
        MsgBox % ActualSku " " PricesBySku[ActualSku]
        Sleep, 1000
    }
}

GetPrices(A_Args[1])
