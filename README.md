# Augment

A toy templating language

## Usage
Compile the binary:
```sh
cargo build --release
```

Then, locate the binary in the `target` directory and move it out.

Provide the file as its argument, and the environment with the `-i` flag. Pipe stdout to a file:
```sh
augment ./index.augment.html -i name="John" > ./index.html
```

## Example
A really scuffed example:
```html
{@keys id name}

<!DOCTYPE html>
<html lang="en">

<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Document</title>
</head>

<body>
  <h1>{title}</h1>
  {count}
  {#if count % 2 = 1}
  yes
  <table>
    <thead>
      <tr>
        <th>id</th>
        <th>name</th>
      </tr>
    </thead>
    <tbody>
      {#for user in users}
      <tr>
        <td>{user[id]}</td>
        <td>{user[name]}</td>
      </tr>
      {/}
    </tbody>
  </table>
  {:else}
    asdfsf
  {/}
</body>

</html>
```
