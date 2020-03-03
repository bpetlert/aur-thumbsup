# Development's Notes

- List repositories

  ```sh
  /usr/bin/pacman-conf -l
  ```

- List all installed packages

  ```sh
  pacman -Q
  ```

- List installed packages in a repository

  ```sh
  /usr/bin/pacman -Sl community | /usr/bin/grep "\[installed\]$"
  ```

- List installed packages not found in sync db(s)

  ```sh
  /usr/bin/pacman -Qm
  ```

- Query user's voted packages

  - need to login
  - sorted by voted
  - `https://aur.archlinux.org/packages/?O=0&SeB=nd&SB=w&SO=d&PP=250&do_Search=Go`
  - `O={offset}` => start=0, step=250

- Test login

```sh
 curl --verbose -L -fs -c /tmp/cookie 'https://aur.archlinux.org/login/?user=<USERNAME>&passwd=<PASSWORD>&remember_me=on' &> /tmp/aur-login.log
```
