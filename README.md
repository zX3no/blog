```
/markdown - stores markdown files that will be compiled
/build - stores the compiled parts of the website
/site - the website which will link to files in /build
/templates - stores the templates to be compiled
/assets - fonts, styles, images, etc.
/src - the compiler
```

TODO:
Maybe markdown files need a heading for the page and a title for the post list. IDK.
Path's should all be set relative to the project, it's kind of all over the place right now. Just look at build/post_list.html

- [x] Better logging
- [x] Allow empty metadata
- [x] Sort posts by date.
- [x] Date posted metadata.
- [x] Summary text is too small and doesn't stand out.
- [x] Deleted files are left in memory.
- [x] Hyperlink colors are weird.
- [x] Re-work the color scheme.
- [x] Add time to logging.
- [x] HTML, CSS & JS minification.
- [ ] Write a package command that combines all files for shipping. Will need to fix paths too!
- [ ] Blurry transform scaling.
- [ ] Syntax highlighting https://github.com/trishume/syntect
- [ ] Add a github logo in the bottom left or right. Mabye a 'Made by Bay ❤' or something.
- [ ] Table of contents on right-side of post.


Nice syntax highting with diffing

https://www.11ty.dev/docs/plugins/syntaxhighlight/