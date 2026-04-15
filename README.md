# recu

CLI tool for tracking recurring expenses.

## Install

```
cargo install recu
```

## Usage

```
recu [COMMAND]
```

| Command    | Description                     |
| ---------- | ------------------------------- |
| `ls`       | List recurring expenses         |
| `add`      | Add a recurring expense         |
| `edit`     | Edit a recurring expense        |
| `rm`       | Remove a recurring expense      |
| `treemap`  | Visualise expenses as a treemap |
| `config`   | Manage configuration            |
| `category` | List or clear expense categories |

By default uses `./recu.csv`; set `RECU_FILE` to override.

### Example

```
$ recu ls
#    name                  amount  due        
───  ────────────────────  ──────  ───────────
@13  iSmog+                 $2.99  in 2 days  
@2   Spookify               $9.99  in 3 days  
@14  Goggle One             $2.99  in 4 days  
@6   Adobo Creative Cloud  $54.99  in 5 days  
@1   Streamberry           $19.99  in 1 week  
@18  Web Hosting            $6.99  in 1 week  
@5   Ghibli+               $11.99  in 1 week  
@9   0Password              $4.99  in 1 week  
@4   Pear TV+              $12.99  in 2 weeks 
@3   ViewTube Premium      $15.99  in 2 weeks 
@16  Gym                   $45.00  in 2 weeks 
@8   Potion                $10.00  in 2 weeks 
@11  GitHug Pro             $4.00  in 3 weeks 
@12  ChatGBT Plus          $20.00  in 3 weeks 
@15  Amazoo Prime          $14.99  in 1 month 
@17  Domain                $15.00  in 6 months
@7   Macrosoft 365         $99.99  in 7 months
@10  FjordVPN              $47.88  in 9 months

Total  $251.46/month  $3017.55/year
```
