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

    ; Go to inventory screen
    Send, {F10}
    Sleep, 1000
    Send, i
    Sleep, 1000

    OutputText := "[`n"
    Loop, parse, SkuStrs, `n 
    {
        ; Clear the screen to a new inventory file
        Send, {Ctrl Down}
        Send, n 
        Send, {Ctrl Up}
        Sleep, 500

        if WinExist("Save changes before proceeding?") {
            Send, {Right}
            Sleep, 100
            Send, {Enter}
            Sleep, 1000

            if WinExist("REIFSNYDER'S AG CENTER - ABC Accounting Client")
                WinActivate
        }

        TrimmedSku := Trim(A_LoopField, OmitChars := "`n`t`r")
        if (TrimmedSku = "") {
            continue
        }

        ControlClick, ThunderRT6TextBox2
        ControlSetText, ThunderRT6TextBox2, %TrimmedSku%
        Send, {Enter}
        Sleep, 500

        ControlGetText, ListPrice, ThunderRT6TextBox27 
        OutputText := OutputText . "    {""sku"": """ . TrimmedSku . """, ""price"": " . ListPrice . "},`n"
    }
    OutputText := SubStr(OutputText, 1, StrLen(OutputText) - 2) . "`n]"
    f := FileOpen(A_ScriptDir . "\exported_bill.json", "w")
    f.Write(OutputText)
    f.Close()
    RunWait, "%A_ScriptDir%\shopify-price-fixer.exe" "%A_ScriptDir%\exported_bill.json"
    FileDelete % A_Args[1]
}

GetPrices(A_Args[1])
