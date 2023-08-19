; Waits for one of any specified images to appear on screen, then return the index 
; of the image that was found first 
; 
; @errors Sets ErrorLevel to 0 if an image was found. ErrorLevel to 1 if no images 
; are found within 60 seconds. ErrorLevel to 2 if one of the images fails to search 
; at all 
; @param ImageFiles {String[]} Array containing the paths to the images to search for 
; @returns {int} Returns -1 if an image failed to search at all or if no images 
; were found in 60 seconds. Otherwise, return the index of the image that was
; found first 
AwaitAnyElementsLoad(ImageFiles) 
{
    ; Pause 250 milliseconds between each interation for 240 iteration = total 
    ; possible wait of 60 seconds 
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

; Wrapper for AwaitAnyElementsLoad that only searches for one ImageFile, rather 
; than a list 
;
; @param ImageFile {String} The location in the filesystem of the image to search 
; for 
; @returns void 
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

