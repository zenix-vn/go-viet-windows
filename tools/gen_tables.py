import re, sys
src = open(sys.argv[1], 'rb').read()
# --- Unicode order of the 186 alpha chars
V = {'a':"aáàảãạ",'â':"âấầẩẫậ",'ă':"ăắằẳẵặ",'e':"eéèẻẽẹ",'ê':"êếềểễệ",'i':"iíìỉĩị",
     'o':"oóòỏõọ",'ô':"ôốồổỗộ",'ơ':"ơớờởỡợ",'u':"uúùủũụ",'ư':"ưứừửữự",'y':"yýỳỷỹỵ"}
order=[]
def vow(k):
    for c in V[k]: order.extend([c.upper(), c])
def cons(s):
    for c in s: order.extend([c.upper(), c])
vow('a');vow('â');vow('ă');cons('bcd');cons('đ');vow('e');vow('ê');cons('fgh');vow('i');cons('jklmn')
vow('o');vow('ô');vow('ơ');cons('pqrst');vow('u');vow('ư');cons('vwx');vow('y');cons('z')
assert len(order)==186, len(order)
# --- TCVN3: first block after "// TCVN3"
i = src.index(b'// TCVN3'); j = src.index(b'0x80', i)
block = src[i:j]
# strip comments
block = re.sub(rb'//[^\n]*', b'', block[len(b'// TCVN3'):])
tc = re.findall(rb"'(.)'", block, re.S)
tc = [b[0] for b in tc]
assert len(tc)==186, len(tc)
# --- VNI-WIN
i = src.index(b'//VNI-WIN'); j = src.index(b'0x0080', i)
vn = [int(x,16) for x in re.findall(rb'0x([0-9a-fA-F]{4})', src[i:j])]
assert len(vn)==186, len(vn)
out = ["// Sinh tự động từ bảng mã của Unikey (vnconv, Phạm Kim Long) bằng tools/gen_tables.py.",
       "// (ký tự Unicode, mã TCVN3 1 byte, mã VNI Windows: byte thấp = chữ gốc, byte cao = dấu)",
       "pub const LEGACY: [(char, u8, u16); 186] = ["]
for u,t,v in zip(order,tc,vn):
    out.append(f"    ('{u}', 0x{t:02X}, 0x{v:04X}),")
out.append("];")
print("\n".join(out))
