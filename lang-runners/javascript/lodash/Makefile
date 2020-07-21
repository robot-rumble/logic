
YARN = yarn -s

# LODASHCATEGORIES = array,collection,number,date,function,lang,object,seq,util

lodash.bundle.js: package.json yarn.lock Makefile
	$(YARN) install
	mkdir -p build
	$(YARN) lodash -d -o lodash.bundle.js
