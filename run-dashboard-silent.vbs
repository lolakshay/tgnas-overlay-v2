Set WshShell = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")
scriptPath = fso.GetParentFolderName(WScript.ScriptFullName)

' Clean up previous instances on port 3000 first
WshShell.Run "cmd.exe /c for /f ""tokens=5"" %a in ('netstat -aon ^| findstr :3000 ^| findstr LISTENING') do taskkill /f /pid %a", 0, true

' Start node server
WshShell.Run "cmd.exe /c cd /d " & Chr(34) & scriptPath & "\web" & Chr(34) & " && node server.js", 0, false
