# Github Commentator

```
pr-commentator 0.2.0
tibo <delor.thibault@gmail.com>
The content comment can be provided in several way. The program will first look for the `comment` arg, if absent try to
get the content from a file specified by the comment-file arg, if absent and use-stdin arg program, it will read from
stdin, otherwise exit unsucessfully

USAGE:
    pr-commentator [FLAGS] [OPTIONS] --comment <Comment> --ref <Git reference> --org <GitHub organization> --repo <Repo name> --token <token>

FLAGS:
        --overwrite    
            Whether previous comment in the PR should be overwritten

        --use-stdin    
            If no comment provided, allow the program to read from stdin

    -h, --help         
            Prints help information

    -V, --version      
            Prints version information


OPTIONS:
        --api-url <Api Url>                      
            The Github api base url

        --comment <Comment>                      
            The content of the comment

        --comment-file <Comment Input File>      
            A file containing the countent of the comment

        --ref <Git reference>
            The reference name to retrieve the PR number (e.g. 'refs/head/my_branch')

        --org <GitHub organization>              
            The Github organization or username containing the repo

        --overwrite-id <Overwrite identifier>
            An arbitrary string used to identify comment to overwrite (e.g commit hash, build number, ...).
                    This imply overwrite mode UsingIdentifier
        --repo-url <Repo Url>
            The repository url, used to deduce the repo name, api url and organization. This is evaluated first if
            present and can be overridden
        --repo <Repo name>                       
            The repository name

        --token <token>                          
            The Github token to use
```