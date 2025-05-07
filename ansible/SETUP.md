# setup (ansible)

* On local (control) machine `pip3 install --user ansible`
* On remote (target) machine, ensure that your local pub ssh key is in the
  `authorized_keys` file on the target
  * `scp ~/.ssh/id_rsa.pub bata.local:/home/greg/.ssh/authorized_keys`
  * On the target `sudo cp ~/.ssh/authorized_keys ~root/.ssh/`
* Confirm host can be reached `ansible myhosts -m ping -i inventory.ini`
