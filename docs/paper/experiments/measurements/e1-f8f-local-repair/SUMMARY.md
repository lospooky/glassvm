# E1 F8-F: recorder finalization failure

Three processes per machine used a synchronous consumer to place a directory
at the staged `recorder-receipt.json` path at `RunFinished`. The recorder then
closed its channels and failed when it attempted to create the receipt file.
All nine runs completed machine execution, evidence fulfillment, and Python
delivery; recorder finalization failed and publication was not attempted.

Each staged run contained an evidence-receipt file and a directory, not a
recorder-receipt file, at the blocked path. Raw outcomes and staged file
inventories were retained before the temporary directories were removed.
This filesystem trigger uses no new production or test-only hook. The
previously approved private `#[cfg(test)]` switch remains a separate
source-level check at an earlier point in the same finalization boundary.
