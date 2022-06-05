.PHONY: default watch clean

default:
	latexmk -shell-escape main.tex -pdf

watch:
	nodemon -x "make" -w main.tex -e tex

clean:
	rm *.aux *.dvi *.fdb_latexmk *.fls *.log *.out *.pdf *.pyg *.toc _minted-*
