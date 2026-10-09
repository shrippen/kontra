import xml.etree.ElementTree as ET
D="logos/05-tab-lane/"
C,Y="#E8DCC4","#FABD2F"
cream=('<polygon points="3,1 5,1 4,12 1,12" fill="{c}"/>'
       '<polygon points="11,1 13,1 15,12 12,12" fill="{c}"/>'
       '<rect x="7" y="1" width="2" height="3" fill="{c}"/>'
              '<rect x="1" y="13" width="14" height="2" fill="{c}"/>')
note='<rect x="6" y="5" width="4" height="6" rx="1" fill="{y}"/>'
for fn,y in (("icon-small.svg",Y),("icon-small-mono.svg",C)):
    s=f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" width="16" height="16">{cream.format(c=C)}{note.format(y=y)}</svg>\n'
    ET.fromstring(s); open(D+fn,"w").write(s)
