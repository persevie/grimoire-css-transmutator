# Easy Migration with Grimoire CSS Transmute (gcsst) Utility

Migrating to Grimoire CSS is simple, thanks to the Grimoire CSS Transmute utility, also known as gcsst. This CLI tool takes the paths of your built CSS files (or the content of built CSS if you’re working in a web environment) and returns a transmuted.json file in the following format:

```json
{
  "classes": [
    {
      "name": "old-class-name",
      "spells": ["spell-1", "spell-2"],
      "oneliner": "spell-1 spell-2"
    }
  ]
}
```

`gcsst` parses the existing CSS using cssparser and automatically generates corresponding spells for each class. One of the standout features of gcsst is the structure of the transmuted.json file, particularly the classes property. It’s designed to look like the structure of a scroll, except for the oneliner property. This makes it incredibly easy to create a scroll or copy-paste the single-line class into your component with minimal effort.

By simplifying the migration process, gcsst helps you move to Grimoire CSS without hassle, and you can instantly start leveraging the power of spells.

> Detailed guides are on the way. Stay tuned for updates!
