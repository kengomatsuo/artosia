# Launches an app, lets it settle, then reports CPU cores used over a window and RAM at its end,
# summed over the whole process tree.
# Usage: powershell -File tools/ram/measure-cpu.ps1 -Exe <path> [-ArgLine "<args>"] [-Settle 15] [-Window 20] [-Label name]
param([string]$Exe,[string]$ArgLine=" ",[string]$Dir=".",[int]$Settle=15,[int]$Window=20,[string]$Label="app")
$p = Start-Process -FilePath $Exe -ArgumentList $ArgLine -WorkingDirectory $Dir -PassThru
Start-Sleep -Seconds $Settle
function Tree($root){
  $all = Get-CimInstance Win32_Process
  function Kids($id){ $all | Where-Object { $_.ParentProcessId -eq $id } | ForEach-Object { $_; Kids $_.ProcessId } }
  @($all | Where-Object { $_.ProcessId -eq $root }) + @(Kids $root)
}
function Cpu($tree){ $s=0; foreach($t in $tree){ $q=Get-Process -Id $t.ProcessId -EA SilentlyContinue; if($q){$s+=$q.TotalProcessorTime.TotalSeconds} }; $s }
$tree = Tree $p.Id
$before = Cpu $tree
Start-Sleep -Seconds $Window
$tree = Tree $p.Id
$after = Cpu $tree
$ws=0;$priv=0
foreach($t in $tree){ $q=Get-Process -Id $t.ProcessId -EA SilentlyContinue; if($q){$ws+=$q.WorkingSet64;$priv+=$q.PrivateMemorySize64} }
"{0}: processes={1} cores={2:N2} workingSetMB={3:N0} privateMB={4:N0}" -f $Label,$tree.Count,(($after-$before)/$Window),($ws/1MB),($priv/1MB)
$tree | ForEach-Object { $q=Get-Process -Id $_.ProcessId -EA SilentlyContinue; if($q){"  {0} ws={1:N0}MB priv={2:N0}MB" -f $q.ProcessName,($q.WorkingSet64/1MB),($q.PrivateMemorySize64/1MB)} }
$tree | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
