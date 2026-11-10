# Launches an app, waits, and sums working set + private bytes over its whole process tree.
# Usage: powershell -File tools/ram/measure.ps1 -Exe <path> [-ArgLine "<args>"] [-Dir <cwd>] [-Wait 40] [-Label name]
param([string]$Exe,[string]$ArgLine=" ",[string]$Dir=".",[int]$Wait=40,[string]$Label="app")
$p = Start-Process -FilePath $Exe -ArgumentList $ArgLine -WorkingDirectory $Dir -PassThru
Start-Sleep -Seconds $Wait
$all = Get-CimInstance Win32_Process
function Kids($id){ $all | Where-Object { $_.ParentProcessId -eq $id } | ForEach-Object { $_; Kids $_.ProcessId } }
$tree = @($all | Where-Object { $_.ProcessId -eq $p.Id }) + @(Kids $p.Id)
$ws=0;$priv=0
foreach($t in $tree){ $q=Get-Process -Id $t.ProcessId -EA SilentlyContinue; if($q){$ws+=$q.WorkingSet64;$priv+=$q.PrivateMemorySize64} }
"{0}: processes={1} workingSetMB={2:N0} privateMB={3:N0}" -f $Label,$tree.Count,($ws/1MB),($priv/1MB)
$tree | ForEach-Object { $q=Get-Process -Id $_.ProcessId -EA SilentlyContinue; if($q){"  {0} ws={1:N0}MB priv={2:N0}MB" -f $q.ProcessName,($q.WorkingSet64/1MB),($q.PrivateMemorySize64/1MB)} }
$tree | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
