AwaitElementLoad(ImageFile)
{
    ImagesArray := []
    ImagesArray[1] := ImageFile
    FoundIndex := AwaitAnyElementsLoad(ImagesArray)

    if (FoundIndex = -1 && ErrorLevel = 2) {
        MsgBox, There was a problem searching for %ImageFile%
    } else if (FoundIndex = -1 && ErrorLevel = 1) {
        MsgBox, Could not find %ImageFile% in 60 seconds 
    }
}

AwaitAnyElementsLoad(ImageFiles) 
{
    loop, 240 
    {
        for i, file in ImageFiles 
        {
            ImageSearch, FoundX, FoundY, 0, 0, A_ScreenWidth, A_ScreenHeight, %file%
            if (ErrorLevel = 2) {
                return -1
            }
            else if (ErrorLevel = 0) 
                return i
        }
        Sleep, 250
    }

    return -1
}

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

