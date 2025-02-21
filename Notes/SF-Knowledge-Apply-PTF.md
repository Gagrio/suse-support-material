== Title ==
 
Applying SUSE Linux Micro kernel hotfixes for SUSE Edge scenarios.
 
== Situation ==
 
A bug of the SUSE Linux Micro kernel was identified and resolved via a hotfix (PTF kernel) provided by L3 Support.
 
== Resolution ==
 
To apply the fix in a SUSE Edge setup, where SUSE Linux Micro is the OS, there are a few different approaches:
1. Download and install the PTF as a transactional update. You will need to reboot. More info here: https://www.suse.com/support/kb/doc/?id=000018572
2. Build a new image for your cluster with the same base image and the PTF kernel applied on it, same as before with Edge Image Builder. You will need to reboot though as applying a kernel on an existing image happens on combustion stage. More info here: https://gitlab.suse.de/-/snippets/2390 (will replace with kb doc when released)
3. Build a new image for your cluster using kiwi and use this image to deploy a new cluster. You do not need to reboot as kiwi will inject the PTF kernel in the image it's building, however it's a more complicated process. More info here: https://gitlab.suse.de/-/snippets/2390 (will replace with kb doc when released)
 
== Cause ==
 
SUSE Linux Micro uses an immutable file system and therefore there are different approaches to installing a hotfix (PTF kernel)
 
== Additional Information ==
 
SUSE Edge solution is based on SUSE Linux Micro where the operating system is immutable therefore allowing us for a few possibilities to get a kernel hotfix deployed.

== Linked Bug Number ==

[Bug #1223600](https://bugzilla.suse.com/show_bug.cgi?id=1223600)