// Authored bounded external-reader experiment (D122), not a translated parser.
// Runs the pinned Toolkit/cwlib RBevel reader locally and projects its fields (floats as
// raw IEEE bits) for parity with lbp-level::bevel; never shipped, never a product
// dependency. Provenance: docs/d122-shape-bevels.md.
import com.google.gson.*;
import cwlib.io.serializer.Serializer;
import cwlib.io.streams.MemoryInputStream;
import cwlib.resources.RBevel;
import cwlib.structs.bevel.BevelVertex;
import cwlib.types.SerializedResource;
import cwlib.types.data.ResourceDescriptor;
import cwlib.types.data.Revision;
import java.io.*;
import java.nio.*;
import java.nio.file.*;
import java.security.MessageDigest;
import java.util.*;

public class BevelOracle {
    static final String PIN="d6a2ef80b5a09999f08f23a803b516096055e5c9";
    static final String SCOPE="bevb_toolkit_rbevel_fields_v1";
    static final int MAX_STORED=64*1024, MAX_EXPANDED=256*1024, MAX_SAMPLES=4096;
    static final long MAX_RUN_BYTES=64L*1024*1024, MAX_SECONDS=600;

    static Map<String,Object> row(Object... f) {
        Map<String,Object> m=new TreeMap<>();for(int i=0;i<f.length;i+=2)m.put((String)f[i],f[i+1]);return m;
    }
    static void need(byte[] b,int at,int n)throws IOException {
        if(at<0||n<0||at>b.length-n)throw new IOException("bounds");
    }
    static int be32(byte[] b,int at)throws IOException {need(b,at,4);return ByteBuffer.wrap(b).getInt(at);}
    static int be16(byte[] b,int at)throws IOException {need(b,at,2);return ByteBuffer.wrap(b).getShort(at)&0xffff;}

    // Independent header/expansion preflight before any community constructor runs
    // (same wrapper rules as lbp-resource; docs/d106-level-oracle.md).
    static Map<String,Object> preflight(byte[] raw)throws IOException {
        if(raw.length>MAX_STORED)throw new IOException("stored budget");
        need(raw,0,12);
        if(raw[0]!='B'||raw[1]!='E'||raw[2]!='V')throw new IOException("not BEV magic");
        if(raw[3]!='b')throw new IOException("only ordinary binary BEVb; encrypted/text blocked");
        int head=be32(raw,4);
        if(head<0x109||head>0x3f8)throw new IOException("head outside oracle scope 0x109..0x3f8");
        int dep=be32(raw,8),p=12,branch=0,branchRev=0,flags=0;boolean compressed=head>=0x132;
        if(head>=0x189) {
            if(head>=0x271){branch=be16(raw,p);branchRev=be16(raw,p+2);p+=4;}
            if(head>=0x297||(head==0x272&&branch==0x4c44&&branchRev>=2)){need(raw,p,1);flags=raw[p++]&255;}
            need(raw,p,1);int c=raw[p++]&255;if(c>1)throw new IOException("compressed bool");compressed=c==1;
        }
        if(dep<p||dep>raw.length-4)throw new IOException("dependency offset bounds");
        long expanded;
        if(compressed) {
            need(raw,p,4);int chunks=be16(raw,p+2);
            if(chunks>0x7fff)throw new IOException("negative chunk count");
            need(raw,p+4,4*chunks);expanded=0;long stored=0;
            for(int i=0;i<chunks;i++){stored+=be16(raw,p+4+4*i);expanded+=be16(raw,p+6+4*i);}
            if(chunks==0)expanded=dep-(p+4);
            if(chunks>0&&stored>dep-(p+4L+4L*chunks))throw new IOException("chunk bytes exceed payload");
        } else expanded=dep-p;
        if(expanded>MAX_EXPANDED)throw new IOException("expanded budget before inflation");
        int count=be32(raw,dep);if(count<0||count>65536)throw new IOException("dependency count budget");
        return row("head",head,"branch_id",branch,"branch_revision",branchRev,"compression_flags",flags,
            "compressed",compressed,"declared_expanded_bytes",expanded);
    }

    static Object desc(ResourceDescriptor d) {
        if(d==null)return null;
        return row("guid",d.isGUID()?String.format(Locale.ROOT,"%08x",d.getGUID().getValue()):null,
            "sha1",d.isHash()?d.getSHA1().toString():null);
    }
    static int bits(float f){return Float.floatToRawIntBits(f);}

    static Map<String,Object> inspect(byte[] raw)throws Exception {
        Map<String,Object> head=preflight(raw);
        SerializedResource res=new SerializedResource(raw);
        Revision rev=res.getRevision();
        if(rev.getHead()!=(int)head.get("head")||(rev.getBranchID()&0xffff)!=(int)head.get("branch_id"))
            throw new IOException("wrapper disagreement");
        MemoryInputStream stream=res.getStream();
        if(stream.getLength()>MAX_EXPANDED)throw new IOException("expanded budget");
        Serializer s=new Serializer(stream,rev);
        RBevel b=s.struct(null,RBevel.class);
        List<Object> mats=new ArrayList<>(),uv=new ArrayList<>(),verts=new ArrayList<>();
        for(int i=0;i<RBevel.MAX_MATERIALS;i++){mats.add(desc(b.getMaterial(i)));uv.add(bits(b.getMaterialUVScale(i)));}
        for(BevelVertex v:b.vertices)verts.add(row("y",bits(v.y),"z",bits(v.z),"rigidity",bits(v.rigidity),
            "smooth_with_previous",(int)v.smoothWithPrevious,"gmat_slot",(int)v.gmatSlot,
            "mapping_mode",v.mappingMode==null?null:v.mappingMode.name()));
        Map<String,Object> r=row("materials",mats,"uv_scales_bits",uv,"auto_smooth_cutoff_angle_bits",bits(b.autoSmoothCutoffAngle),
            "vertices",verts,"include_backface",b.includeBackface,"smooth_with_front",b.smoothWithFront,
            "relax_strength_bits",bits(b.relaxStrength),"sub_div_radius_bits",bits(b.subDivRadius),
            "fixed_bevel_size_bits",bits(b.fixedBevelSize),"fixed_bevel_size",b.fixedBevelSize,"spongy",b.spongy,
            "soft_physics_settings",desc(b.softPhysicsSettings),"texture_repeats_bits",bits(b.textureRepeats),
            "consumed_bytes",s.getOffset(),"expanded_bytes",stream.getLength(),"trailing_bytes",stream.getLength()-s.getOffset());
        r.putAll(head);
        return r;
    }

    // ---- authored synthetic guards (no game bytes) ----
    static byte[] header(int head,int flags,byte[] payload) {
        ByteBuffer b=ByteBuffer.allocate(32+payload.length);
        b.put(new byte[]{'B','E','V','b'}).putInt(head);int depAt=b.position();b.putInt(0);
        if(head>=0x271)b.putShort((short)0).putShort((short)0);
        if(head>=0x297)b.put((byte)flags);
        b.put((byte)0).put(payload);
        int dep=b.position();b.putInt(0);b.putInt(depAt,dep);
        return Arrays.copyOf(b.array(),b.position());
    }
    static void rejected(byte[] raw,String why)throws Exception {
        try{preflight(raw);}catch(IOException expected){return;}
        throw new AssertionError("accepted "+why);
    }
    static void selfTest()throws Exception {
        // Synthetic RBevel: 4 null materials, 4 UV scales, 1 vertex, tail. Uncompressed ints.
        ByteBuffer p=ByteBuffer.allocate(128);
        p.putInt(4);for(int i=0;i<4;i++)p.putInt(0).put((byte)0);
        p.putInt(4);for(int i=0;i<4;i++)p.putFloat(1f);
        p.putFloat(45f).putInt(1).putFloat(0.5f).putFloat(-1f).putFloat(1f).put((byte)1).put((byte)2).put((byte)3);
        p.put((byte)1).put((byte)0).putFloat(0f).putFloat(0.3f).putFloat(10f).put((byte)0).putInt(0).put((byte)0).putFloat(1f);
        byte[] ok=header(0x3f8,0,Arrays.copyOf(p.array(),p.position()));
        Map<String,Object> r=inspect(ok);
        if(!r.get("head").equals(0x3f8)||!r.get("trailing_bytes").equals(0)||((List<?>)r.get("vertices")).size()!=1
            ||!r.get("fixed_bevel_size_bits").equals(bits(10f)))throw new AssertionError("synthetic bevel "+r);
        for(int n=0;n<ok.length;n++)rejected(Arrays.copyOf(ok,n),"truncation "+n);
        byte[] bad=ok.clone();bad[3]='e';rejected(bad,"encrypted");
        bad=ok.clone();bad[0]='P';rejected(bad,"wrong magic");
        rejected(header(0x3f9,0,new byte[4]),"head above LBP2 max");
        System.out.println("Bevel synthetic header/truncation/encrypted/head guards passed");
    }

    static String sha1(byte[] b)throws Exception {return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-1").digest(b));}
    static boolean sameArchive(String sourcePath,Path archive) {
        String name=archive.getFileName().toString().toLowerCase(Locale.ROOT);
        return sourcePath.startsWith("USRDIR/")&&sourcePath.toLowerCase(Locale.ROOT).endsWith("/"+name);
    }
    static void inc(Map<String,Long> m,String k){m.merge(k,1L,Long::sum);}

    public static void main(String[] args)throws Exception {
        if(args.length==1&&args[0].equals("--self-test")){selfTest();return;}
        if(args.length<3)throw new IllegalArgumentException("MANIFEST NEW-JSON ARCHIVE...");
        Path output=Path.of(args[1]).toAbsolutePath();
        if(!output.toString().endsWith(".json")||Files.exists(output)||
            !output.getParent().toRealPath().startsWith(Path.of("local-analysis").toRealPath()))throw new IOException("new ignored JSON required");
        JsonObject manifest=JsonParser.parseString(Files.readString(Path.of(args[0]))).getAsJsonObject();
        if(!manifest.get("toolkit_commit").getAsString().equals(PIN)||!manifest.get("scope").getAsString().equals(SCOPE))
            throw new IOException("manifest pin/scope differs");
        JsonArray samples=manifest.getAsJsonArray("samples");if(samples.size()>MAX_SAMPLES)throw new IOException("sample budget");
        List<Path> archives=new ArrayList<>();for(int i=2;i<args.length;i++)archives.add(Path.of(args[i]));
        Map<Path,RandomAccessFile> open=new HashMap<>();
        List<Object> rows=new ArrayList<>();long total=0,deadline=System.nanoTime()+MAX_SECONDS*1_000_000_000L;
        Map<String,Long> status=new TreeMap<>();
        try {
            for(JsonElement item:samples) {
                if(System.nanoTime()>deadline)throw new IOException("oracle time budget");
                JsonObject q=item.getAsJsonObject();String sha=q.get("sha1").getAsString();
                String src=q.get("source_path").getAsString();
                long at=q.get("offset").getAsLong(),size=q.get("size").getAsLong();
                Map<String,Object> r=row("sha1",sha,"source_path",src,"offset",at,"stored_bytes",size,
                    "guid",q.get("guid").isJsonNull()?null:q.get("guid").getAsString());
                rows.add(r);
                try {
                    List<Path> match=new ArrayList<>();for(Path a:archives)if(sameArchive(src,a))match.add(a);
                    if(match.size()!=1)throw new IOException("archive not supplied or ambiguous: "+src);
                    RandomAccessFile f=open.get(match.get(0));
                    if(f==null){f=new RandomAccessFile(match.get(0).toFile(),"r");open.put(match.get(0),f);}
                    if(at<0||size<12||size>MAX_STORED||at>f.length()-size||total+size>MAX_RUN_BYTES)throw new IOException("source/read budget");
                    byte[] raw=new byte[(int)size];f.seek(at);f.readFully(raw);total+=size;
                    if(!sha1(raw).equals(sha))throw new IOException("identifier mismatch");
                    r.putAll(inspect(raw));
                    r.put("status",((int)r.get("trailing_bytes"))==0?"parsed_exact":"parsed_trailing_bytes");
                }catch(Exception e){
                    String msg=e.getMessage();if(msg!=null&&msg.length()>300)msg=msg.substring(0,300);
                    r.putAll(row("status",e instanceof IOException?"rejected":"failed","reason",e.getClass().getSimpleName()+": "+msg));
                }
                inc(status,(String)r.get("status"));
            }
        } finally {for(RandomAccessFile f:open.values())f.close();}
        Files.writeString(output,new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(row(
            "schema_version",1,"toolkit_commit",PIN,"scope",SCOPE,"sample_bytes",total,"status_counts",status,
            "float_note","floats are raw IEEE-754 bits (Float.floatToRawIntBits) as signed int",
            "bevels",rows)),StandardOpenOption.CREATE_NEW);
        System.out.println("Bevel oracle: "+rows.size()+" bevels, "+total+" archived bytes, "+status);
    }
}
