# Github Commentator

```
pr-commentator 0.1.0
tibo <delor.thibault@gmail.com>
The content comment can be provided in several way. The program will first look for the `comment` arg, if absent try to
get the content from a file specified by the comment-file arg, if absent and use-stdin arg program, it will read from
stdin, otherwise exit unsucessfully

USAGE:
    pr-commentator [FLAGS] [OPTIONS] --branch <Branch> --comment <Comment> --org <GitHub organization> --repo <Repo name> --token <token>

FLAGS:
        --use-stdin    
            If no comment provided, allow the program to read from stdin

    -h, --help         
            Prints help information

    -V, --version      
            Prints version information


OPTIONS:
        --api-url <Api Url>                    
            The Github api base url [default: https://api.github.com/]

    -b, --branch <Branch>                      
            The branch name to retrieve the PR number

        --comment <Comment>                    
            The content of the comment

        --comment-file <Comment Input File>    
            A file containing the countent of the comment

        --org <GitHub organization>            
            The Github organisation or username containing the repo

        --repo <Repo name>                     
            The repository name

        --token <token>                        
            The Github token to use
```