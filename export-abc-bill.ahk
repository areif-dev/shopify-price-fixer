+#e::
#NoEnv  ; Recommended for performance and compatibility with future AutoHotkey releases.
; #Warn  ; Enable warnings to assist with detecting common errors.
SendMode Input  ; Recommended for new scripts due to its superior speed and reliability.
SetWorkingDir %A_ScriptDir%  ; Ensures a consistent starting directory.

if WinExist("REIFSNYDER'S AG CENTER - ABC Accounting Client")
    WinActivate
else 
    return

Send, {PgDn}{PgUp}{PgUp}
ProductSkus := []
ProductPrices := []
loop {

    Sleep, 500
    ControlGetFocus, FocusedBox
    ControlGetText, ProductSku, %FocusedBox%

    ; End execution because the end of the file has been reached
    if (ProductSku = ""){
        msgbox "Ended"
        break
    }

    Send, {ALT DOWN}1{ALT UP}
    Sleep, 500
    ControlGetText, ProductPrice, ThunderRT6TextBox27

    if (ProductPrice <= 0) {
        continue
    }

    ProductSkus.Push(ProductSku)
    ProductPrices.Push(ProductPrice)
    Send, {F9}
    Sleep, 500

    Send, {Left}{Left}{Down}
}

OutputText := "[`n"
for key, val in ProductSkus
{
    OutputText := OutputText . "    {""sku"": """ . val . """, ""price"": " . ProductPrices[key] . "},`n"
}

OutputText := SubStr(OutputText, 1, StrLen(OutputText) - 2) . "`n]"
FileAppend, %OutputText%, %A_Desktop%\exported_bill_%A_YYYY%-%A_MM%-%A_DD%T%A_Hour%-%A_Min%.json
return
