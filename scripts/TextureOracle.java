// Authored C1 adapter; external pinned Toolkit, no implementation translated.
// Provenance: docs/textures.md. Only allowlisted clear TEX /GTF wrappers.
// Original inputs read-only; output contains metadata/digests, never asset bytes.
import com.google.gson.*;
import cwlib.types.SerializedResource;
import cwlib.resources.RTexture;
import cwlib.structs.texture.CellGcmTexture;
import cwlib.external.DDSReader;
import java.io.*;
import java.nio.file.*;
import java.security.MessageDigest;
import java.awt.image.BufferedImage;
import java.util.*;
import java.util.zip.Inflater;

public class TextureOracle {
    static int be16(byte[] b,int p) { return (b[p]&255)*256+(b[p+1]&255); }
    static long le32(byte[] b,int p) {return Integer.toUnsignedLong(java.nio.ByteBuffer.wrap(b,p,4).order(java.nio.ByteOrder.LITTLE_ENDIAN).getInt());}
    static Map<String,Object> row(Object... fields) {
        Map<String,Object> m=new TreeMap<>();for(int i=0;i<fields.length;i+=2)m.put((String)fields[i],fields[i+1]);return m;
    }
    static String sha(byte[] b) throws Exception {return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(b));}
    // Independent strict experiment guard, not Toolkit's forgiving inflateData.
    static byte[] clearPayload(byte[] raw) throws Exception {
        if(raw.length<8)throw new IOException("truncated wrapper");
        String magic=new String(raw,0,4,java.nio.charset.StandardCharsets.US_ASCII);
        if(!magic.equals("TEX ")&&!magic.equals("GTF "))throw new IOException("not allowlisted clear texture");
        int p=magic.equals("GTF ")?28:4;
        if(raw.length<p+4)throw new IOException("truncated header");
        int count=be16(raw,p+2), data=p+4+4*count;
        if(count>32767||data>raw.length)throw new IOException("chunk table budget/bounds");
        if(count==0){if(raw.length-data>32*1024*1024)throw new IOException("raw budget");return Arrays.copyOfRange(raw,data,raw.length);}
        long expanded=0,stored=0;
        for(int i=0;i<count;i++){stored+=be16(raw,p+4+i*4);expanded+=be16(raw,p+6+i*4);}
        if(expanded>32*1024*1024||stored!=raw.length-data)throw new IOException("expanded budget or unaccounted payload bytes");
        ByteArrayOutputStream out=new ByteArrayOutputStream((int)expanded);
        for(int i=0;i<count;i++){
            int s=be16(raw,p+4+i*4),e=be16(raw,p+6+i*4);
            if(s==e)out.write(raw,data,s);
            else {
                Inflater z=new Inflater();
                try {
                    z.setInput(raw,data,s);byte[] b=new byte[e+1];int n=0;
                    while(!z.finished()&&n<b.length){int got=z.inflate(b,n,b.length-n);if(got==0&&!z.finished())throw new IOException("stalled/truncated zlib");n+=got;}
                    if(n!=e||!z.finished()||z.getRemaining()!=0)throw new IOException("zlib length/end/trailing mismatch");
                    out.write(b,0,n);
                } finally {z.end();}
            }
            data+=s;
        }
        return out.toByteArray();
    }
    static void pixelBudget(long w,long h) throws IOException {
        if(w<1||h<1||w>4096||h>4096||w*h>4*1024*1024)throw new IOException("pixel/dimension budget");
    }
    static void rejected(byte[] b) throws Exception {
        try {clearPayload(b);}catch(IOException e){return;}
        throw new AssertionError("invalid synthetic wrapper accepted");
    }
    static void selfTest() throws Exception {
        byte[] plain="authored synthetic chunk".getBytes(java.nio.charset.StandardCharsets.US_ASCII);
        ByteArrayOutputStream compressed=new ByteArrayOutputStream();
        try(java.util.zip.DeflaterOutputStream z=new java.util.zip.DeflaterOutputStream(compressed)){z.write(plain);}
        byte[] c=compressed.toByteArray(),raw=new byte[12+c.length];
        java.nio.ByteBuffer b=java.nio.ByteBuffer.wrap(raw);
        b.put(new byte[]{'T','E','X',' '}).putShort((short)1).putShort((short)1).putShort((short)c.length).putShort((short)plain.length).put(c);
        if(!Arrays.equals(clearPayload(raw),plain))throw new AssertionError("zlib result");
        rejected(Arrays.copyOf(raw,raw.length-1));
        byte[] bad=raw.clone();bad[11]--;rejected(bad);bad=raw.clone();bad[11]++;rejected(bad);
        rejected(new byte[]{'G','T','F','e',0,0,0,0});
        byte[] stored=new byte[12+plain.length];b=java.nio.ByteBuffer.wrap(stored);
        b.put(new byte[]{'T','E','X',' '}).putShort((short)1).putShort((short)1).putShort((short)plain.length).putShort((short)plain.length).put(plain);
        if(!Arrays.equals(clearPayload(stored),plain))throw new AssertionError("stored result");
        byte[] zero=new byte[8+plain.length];System.arraycopy(new byte[]{'T','E','X',' '},0,zero,0,4);System.arraycopy(plain,0,zero,8,plain.length);
        if(!Arrays.equals(clearPayload(zero),plain))throw new AssertionError("zero chunks result");
        // Original synthetic DDS recipe: 8x8 BC1 red, then 4x4 green, 2x2/1x1 blue.
        byte[] dds=new byte[128+32+8+8+8];b=java.nio.ByteBuffer.wrap(dds).order(java.nio.ByteOrder.LITTLE_ENDIAN);
        b.put(new byte[]{'D','D','S',' '});b.putInt(4,124);b.putInt(12,8);b.putInt(16,8);b.putInt(28,4);b.putInt(76,32);b.putInt(80,4);
        System.arraycopy(new byte[]{'D','X','T','1'},0,dds,84,4);
        for(int p=128;p<160;p+=8)b.putShort(p,(short)0xf800);b.putShort(160,(short)0x07e0);b.putShort(168,(short)0x001f);b.putShort(176,(short)0x001f);
        if(DDSReader.read(dds,DDSReader.ARGB,0)[0]!=0xffff0000)throw new AssertionError("synthetic top-mip red");
        b.putShort(128,(short)0x18e3);
        int endpoint=DDSReader.read(dds,DDSReader.ARGB,0)[0];
        if(endpoint!=0xff191c19)throw new AssertionError("low endpoint oracle changed");
        b.putShort(128,(short)0xf800);
        System.out.println("Synthetic RGB565 0x18e3 endpoint ARGB: "+Integer.toHexString(endpoint));
        // Original BC3 block: all alpha indices, reversed color endpoints, four colors.
        byte[] bc3=Arrays.copyOf(dds,144);
        java.nio.ByteBuffer v=java.nio.ByteBuffer.wrap(bc3).order(java.nio.ByteOrder.LITTLE_ENDIAN);
        v.putInt(12,4);v.putInt(16,4);v.putInt(28,1);
        System.arraycopy(new byte[]{'D','X','T','5'},0,bc3,84,4);
        long alphaIndices=0;for(int i=0;i<16;i++)alphaIndices|=(long)(i%8)<<(3*i);
        for(int i=0;i<6;i++)bc3[130+i]=(byte)(alphaIndices>>(i*8));
        v.putShort(136,(short)0xf800);v.putShort(138,(short)0x07e0);v.putInt(140,0xe4e4e4e4);
        int[][] modes={{255,0,255,0,218,182,145,109,72,36},{0,255,0,255,51,102,153,204,0,255},
            {17,17,17,17,17,17,17,17,0,255},{23,11,23,11,21,19,17,16,14,12}};
        int[] colors={0xff0000,0x00ff00,0xaa5500,0x55aa00};
        for(int[] mode:modes){
            bc3[128]=(byte)mode[0];bc3[129]=(byte)mode[1];
            int[] pixels=DDSReader.read(bc3,DDSReader.ARGB,0);
            for(int i=0;i<16;i++)if(pixels[i]!=((mode[2+i%8]<<24)|colors[i%4]))throw new AssertionError("BC3 alpha/color oracle mode "+mode[0]+","+mode[1]+" pixel "+i+" actual="+Integer.toHexString(pixels[i])+" expected="+Integer.toHexString((mode[2+i%8]<<24)|colors[i%4]));
        }
        v.putShort(136,(short)0x07e0);v.putShort(138,(short)0xf800);
        bc3[128]=(byte)255;bc3[129]=0;
        int[] reversed=DDSReader.read(bc3,DDSReader.ARGB,0);
        if(reversed[2]!=0xda7f7f00||reversed[3]!=0)throw new AssertionError("known BC3 reversed-color conflict changed");
        System.out.println("Synthetic BC3 alpha modes/equality passed; documented conflict: reversed BC3 colors use BC1 three-color mode in Toolkit");
        String outcome;
        try {int pixel=DDSReader.read(dds,DDSReader.ARGB,1)[0];outcome=pixel==0xff00ff00?"correct_green":"incorrect_pixel";}
        catch(RuntimeException e){outcome=e.getClass().getSimpleName();}
        System.out.println("Synthetic guards passed; Toolkit BC1 top mip red passed; 8x8 mip 1 oracle outcome: "+outcome);
    }
    public static void main(String[] args) throws Exception {
        if(args.length==1&&args[0].equals("--self-test")){selfTest();return;}
        if(args.length!=3)throw new IllegalArgumentException("FARC manifest NEW-JSON");
        Path output=Path.of(args[2]).toAbsolutePath();
        if(!output.getParent().toRealPath().startsWith(Path.of("local-analysis").toRealPath())||Files.exists(output))throw new IOException("new ignored output required");
        JsonObject manifest=JsonParser.parseString(Files.readString(Path.of(args[1]))).getAsJsonObject();
        if(manifest.getAsJsonArray("samples").size()>48)throw new IOException("sample count budget");
        List<Object> results=new ArrayList<>();long total=0;
        try(RandomAccessFile farc=new RandomAccessFile(args[0],"r")){
            for(JsonElement item:manifest.getAsJsonArray("samples")){
                JsonObject sample=item.getAsJsonObject();String id=sample.get("sha1").getAsString();
                Map<String,Object> result=row("sha1",id);results.add(result);
                try {
                    long offset=sample.get("offset").getAsLong(),size=sample.get("size").getAsLong();
                    if(offset<0||size<8||size>16*1024*1024||offset>farc.length()-size||total+size>128*1024*1024)throw new IOException("sample range/global budget");
                    // Manifest locations originate in the validated census; recheck table-independent content hash.
                    byte[] raw=new byte[(int)size];farc.seek(offset);farc.readFully(raw);total+=size;
                    String actual=HexFormat.of().formatHex(MessageDigest.getInstance("SHA-1").digest(raw));
                    if(!actual.equals(id))throw new IOException("FARC payload SHA-1 mismatch");
                    byte[] strict=clearPayload(raw);
                    if(raw[0]=='G'){
                        pixelBudget(be16(raw,12),be16(raw,14));
                        if(raw[6]!=2||raw[7]!=0||be16(raw,16)!=1)throw new IOException("non-simple 2D GTF excluded");
                    } else {
                        if(strict.length<128||!Arrays.equals(Arrays.copyOf(strict,4),new byte[]{'D','D','S',' '}))throw new IOException("missing DDS header");
                        pixelBudget(le32(strict,16),le32(strict,12));
                        if((le32(strict,112)&0x0020fe00L)!=0)throw new IOException("DDS cube/volume excluded");
                    }
                    SerializedResource serialized=new SerializedResource(raw);
                    if(!Arrays.equals(strict,serialized.getStream().getBuffer()))throw new IOException("strict vs Toolkit payload mismatch");
                    RTexture tex=new RTexture(serialized);CellGcmTexture info=tex.getInfo();
                    if(info!=null)result.put("gtf",row("width",info.getWidth(),"height",info.getHeight(),"depth",info.getDepth(),
                        "mips",info.getMipCount(),"format_base",info.getFormat().getValue(),"cubemap",info.isCubemap(),"volume",info.isVolumeTexture()));
                    byte[] dds=tex.getDDSFileData();BufferedImage image=tex.getImage();
                    if(dds==null||image==null)throw new IOException("Toolkit returned no DDS/image");
                    MessageDigest pixels=MessageDigest.getInstance("SHA-256");
                    for(int y=0;y<image.getHeight();y++)for(int x=0;x<image.getWidth();x++){
                        int c=image.getRGB(x,y);pixels.update(new byte[]{(byte)(c>>>16),(byte)(c>>>8),(byte)c,(byte)(c>>>24)});
                    }
                    result.put("dds",row("width",le32(dds,16),"height",le32(dds,12),"mips",le32(dds,28),
                        "fourcc",new String(dds,84,4,java.nio.charset.StandardCharsets.US_ASCII),"rgb_bits",le32(dds,88),"caps2",le32(dds,112),"byte_count",dds.length,
                        "flags",le32(dds,8),"pitch_or_linear_size",le32(dds,20),"depth",le32(dds,24),"pixel_flags",le32(dds,80),
                        "masks",new long[]{le32(dds,92),le32(dds,96),le32(dds,100),le32(dds,104)},
                        "caps",new long[]{le32(dds,108),le32(dds,112),le32(dds,116),le32(dds,120)}));
                    result.put("payload_sha256",sha(strict));result.put("dds_sha256",sha(dds));
                    result.put("top_mip_rgba_sha256",HexFormat.of().formatHex(pixels.digest()));
                    result.put("image_width",image.getWidth());result.put("image_height",image.getHeight());result.put("no_srgb",tex.noSRGB);
                    result.put("status","decoded_top_mip");
                } catch(Exception e){result.put("status","failed_or_excluded");result.put("reason",e.toString());}
            }
        }
        try(Writer w=Files.newBufferedWriter(output,StandardOpenOption.CREATE_NEW)){
            new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(row("schema_version",1,
                "toolkit_commit","d6a2ef80b5a09999f08f23a803b516096055e5c9","asset_bytes_read",total,"samples",results),w);
        }
        System.out.println("Texture oracle: "+results.size()+" samples, "+total+" archived bytes; metadata/digests only");
    }
}
