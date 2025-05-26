# Template Extensions

## Example

### input.eure

```eure
@ values.for.documentA
$template-input.of = ./documentA.eure
key1 = "value1"
key2 = "value2"

@ values.for.documentB
$template-input.of = ./documentB.eure
key1 = "value3"
key2 = "value4"
```

### documentA.eure

```eure
key1.$template.include.path = .key1
key2.$template.include.path = .key2
```

### documentB.eure

```eure
key1.$template.include.path = .key1
key2.$template.include.path = .key2
```

## $template-value.for

Mark the object as a value set for a template.

## $template.if

Deactivate the object if the condition is false.

## $template.for_each

Make array of objects for each value

## $template.include = { path, type }
