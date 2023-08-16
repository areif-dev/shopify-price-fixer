#NoEnv  ; Recommended for performance and compatibility with future AutoHotkey releases.
; #Warn  ; Enable warnings to assist with detecting common errors.
SendMode Input  ; Recommended for new scripts due to its superior speed and reliability.
SetWorkingDir %A_ScriptDir%  ; Ensures a consistent starting directory.

#Include lib.ahk

GetPrices(SkusFileLocation) {
    ShortWait = 100
    ScreenDir = %A_ScriptDir%\screens\
    InventoryScreens := []
    InventoryScreens[1] := ScreenDir . "save_as_popup.png" 
    InventoryScreens[2] := ScreenDir . "empty_inventory_screen.png"

    FileRead, SkuStrs, %SkusFileLocation%

    if WinExist("REIFSNYDER'S AG CENTER - ABC Accounting Client")
        WinActivate
    else 
        Run, "C:\ABC Software\Client4\abctwin.exe"

    ; Go to inventory screen
    Send, {F10}
    AwaitElementLoad(ScreenDir . "selection_screen.png")
    Send, i
    AwaitElementLoad(ScreenDir . "inventory_screen.png")

    f := FileOpen(A_ScriptDir . "\exported_bill.json", "w")
    f.Write("")
    f.Close()

    f := FileOpen(A_ScriptDir . "\exported_bill.json", "a")

    OutputText := "["
    Loop, parse, SkuStrs, `n 
    {
        if (A_Index = 1) {
            OutputText := OutputText . "`n"
        } else {
            OutputText := OutputText . ",`n"
        }

        ; Clear the screen to a new inventory file
        Send, {Ctrl Down}
        Sleep % ShortWait * 2
        Send, n 
        Sleep % ShortWait * 2
        Send, {Ctrl Up}
        foundScreenIndex := AwaitAnyElementsLoad(InventoryScreens)

        if (foundScreenIndex = -1) {
            MsgBox, Inventory screen did not clear after Ctrl+N
            return 
        } else if (foundScreen = 1) {
            Send, {Right}
            Sleep % SortWait * 2 
            Send, {Enter}
            Sleep % ShortWait * 2

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

        AwaitElementLoad(ScreenDir . "complete_inventory_screen.png")
        ControlGetText, ListPrice, ThunderRT6TextBox27 
        OutputText := OutputText . "    {""sku"": """ . TrimmedSku . """, ""price"": " . ListPrice . "}"
        f.Write(OutputText)
        OutputText := ""
    }

    f.Write("`n]")
    f.Close()
    RunWait, "%A_ScriptDir%\shopify-price-fixer.exe" "%A_ScriptDir%\exported_bill.json"
    FileDelete % A_Args[1]
}

GetPrices(A_Args[1])
