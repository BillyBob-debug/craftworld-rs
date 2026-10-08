// Authored bounded external-reader experiment (D106), not a translated parser.
// Runs the pinned Toolkit/cwlib RLevel reader locally and projects a summary;
// never shipped, never a product dependency. Provenance: docs/d106-level-oracle.md.
import com.google.gson.*;
import cwlib.enums.Part;
import cwlib.io.serializer.Serializer;
import cwlib.resources.RLevel;
import cwlib.structs.things.Thing;
import cwlib.structs.things.components.shapes.Polygon;
import cwlib.structs.things.parts.*;
import cwlib.types.SerializedResource;
import cwlib.types.data.GUID;
import cwlib.types.data.ResourceDescriptor;
import cwlib.types.data.Revision;
import cwlib.io.streams.MemoryInputStream;
import org.joml.Matrix4f;
import org.joml.Vector3f;
import org.joml.Vector4f;
import java.io.*;
import java.lang.reflect.Field;
import java.nio.*;
import java.nio.file.*;
import java.security.MessageDigest;
import java.util.*;

public class LevelOracle {
    static final String PIN="d6a2ef80b5a09999f08f23a803b516096055e5c9";
    static final String SCOPE="lvlb_toolkit_rlevel_summary_and_scene_v1";
    static final int MAX_STORED=16*1024*1024, MAX_EXPANDED=64*1024*1024, MAX_SAMPLES=1024;
    static final long MAX_RUN_BYTES=1024L*1024*1024, MAX_SECONDS=3600;
    static final int MAX_POLYGON_VERTICES=1<<20;

    static Map<String,Object> row(Object... f) {
        Map<String,Object> m=new TreeMap<>();for(int i=0;i<f.length;i+=2)m.put((String)f[i],f[i+1]);return m;
    }
    static void need(byte[] b,int at,int n)throws IOException {
        if(at<0||n<0||at>b.length-n)throw new IOException("bounds");
    }
    static int be32(byte[] b,int at)throws IOException {need(b,at,4);return ByteBuffer.wrap(b).getInt(at);}
    static int be16(byte[] b,int at)throws IOException {need(b,at,2);return ByteBuffer.wrap(b).getShort(at)&0xffff;}

    // Independent header/expansion preflight before any community constructor runs.
    // Layout per docs/d106-level-oracle.md (wrapper rules already used by lbp-resource).
    static Map<String,Object> preflight(byte[] raw)throws IOException {
        if(raw.length>MAX_STORED)throw new IOException("stored budget");
        need(raw,0,12);
        if(raw[0]!='L'||raw[1]!='V'||raw[2]!='L')throw new IOException("not LVL magic");
        if(raw[3]!='b')throw new IOException("only ordinary binary LVLb; encrypted/text blocked");
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
            "compressed",compressed,"declared_expanded_bytes",expanded,"dependency_count_raw",count);
    }

    // Serializer subclass only observes nesting through the public log hook.
    static class DepthSerializer extends Serializer {
        static Field things,visited;
        int maxDepth;
        DepthSerializer(MemoryInputStream s,Revision r){super(s,r);}
        @Override public void log(String message,int level) {
            try{int d=((Stack<?>)things.get(this)).size();if(d>maxDepth)maxDepth=d;}
            catch(IllegalAccessException e){throw new IllegalStateException(e);}
        }
        @SuppressWarnings("unchecked") TreeMap<Integer,Object> tags()throws IllegalAccessException {
            return new TreeMap<>((Map<Integer,Object>)visited.get(this));
        }
    }
    static {
        try {
            DepthSerializer.things=Serializer.class.getDeclaredField("things");DepthSerializer.things.setAccessible(true);
            DepthSerializer.visited=Serializer.class.getDeclaredField("visited");DepthSerializer.visited.setAccessible(true);
        }catch(ReflectiveOperationException e){throw new ExceptionInInitializerError(e);}
    }

    static String guid(GUID g){return g==null?null:String.format(Locale.ROOT,"%08x",g.getValue());}
    static Object desc(ResourceDescriptor d) {
        if(d==null)return null;
        return row("guid",d.isGUID()?guid(d.getGUID()):null,"sha1",d.isHash()?d.getSHA1().toString():null);
    }
    static String descKey(ResourceDescriptor d) {
        if(d==null)return "null";
        if(d.isGUID())return "g"+guid(d.getGUID());
        if(d.isHash())return "h"+d.getSHA1();
        return "invalid";
    }
    static List<Float> m44(Matrix4f m){if(m==null)return null;float[] f=m.get(new float[16]);List<Float> l=new ArrayList<>();for(float v:f)l.add(v);return l;}
    static List<Float> v4(Vector4f v){return v==null?null:List.of(v.x,v.y,v.z,v.w);}
    static List<Float> v3(Vector3f v){return v==null?null:List.of(v.x,v.y,v.z);}
    static void inc(Map<String,Long> m,String k){m.merge(k,1L,Long::sum);}

    static Map<String,Object> thingRef(Thing t,IdentityHashMap<Object,Integer> tag) {
        return t==null?null:row("tag",tag.get(t),"uid",t.UID);
    }
    static List<String> parts(Thing t) {
        List<String> l=new ArrayList<>();
        for(Part p:Part.values())if(p.getSerializable()!=null&&t.hasPart(p))l.add(p.name());
        return l;
    }
    static Map<String,Object> scene(Thing t,IdentityHashMap<Object,Integer> tag) {
        Map<String,Object> r=row("tag",tag.get(t),"uid",t.UID,"parent",thingRef(t.parent,tag),
            "group_head",thingRef(t.groupHead,tag),"old_emitter",thingRef(t.oldEmitter,tag),
            "plan_guid",guid(t.planGUID),"flags",(int)t.flags,"parts",parts(t));
        PPos pos=t.getPart(Part.POS);
        if(pos!=null)r.put("pos",row("world_m44_column_major",m44(pos.worldPosition),
            "local_m44_column_major_toolkit_recomputed",m44(pos.localPosition),
            "bone_of",thingRef(pos.thingOfWhichIAmABone,tag),"anim_hash",pos.animHash));
        PRenderMesh rm=t.getPart(Part.RENDER_MESH);
        if(rm!=null) {
            List<Object> bones=new ArrayList<>();
            if(rm.boneThings!=null)for(Thing b:rm.boneThings)bones.add(thingRef(b,tag));
            r.put("render_mesh",row("mesh",desc(rm.mesh),"bone_count",bones.size(),"bones",bones,
                "anim",desc(rm.anim),"editor_color",rm.editorColor,"visibility_flags",(int)rm.visibilityFlags,
                "cast_shadows",rm.castShadows==null?null:rm.castShadows.name()));
        }
        PJoint jt=t.getPart(Part.JOINT);
        if(jt!=null) {
            List<Integer> bi=new ArrayList<>();if(jt.boneIdx!=null)for(int v:jt.boneIdx)bi.add(v);
            r.put("joint",row("a",thingRef(jt.a,tag),"b",thingRef(jt.b,tag),"a_contact",v3(jt.aContact),
                "b_contact",v3(jt.bContact),"length",jt.length,"angle",jt.angle,"settings",desc(jt.settings),
                "bone_idx",bi,"bone_lengths",v4(jt.boneLengths),"type",jt.type,"render_scale",jt.renderScale,
                "hide_in_play_mode",jt.hideInPlayMode));
        }
        PShape sh=t.getPart(Part.SHAPE);
        if(sh!=null) {
            Polygon poly=sh.polygon;List<Object> verts=new ArrayList<>();int[] loops=new int[0];boolean z=false;
            if(poly!=null){z=poly.requiresZ;loops=poly.loops==null?new int[0]:poly.loops;
                if(poly.vertices!=null)for(Vector3f v:poly.vertices)verts.add(v==null?null:List.of(v.x,v.y,v.z));}
            List<Integer> lp=new ArrayList<>();for(int v:loops)lp.add(v);
            r.put("shape",row("vertex_count",verts.size(),"requires_z",z,"loops",lp,"vertices",verts,
                "material",desc(sh.material),"old_material",desc(sh.oldMaterial),"thickness",sh.thickness,
                "bevel_size",sh.bevelSize,"flags",(int)sh.flags,"color",sh.color,"brightness",sh.brightness,
                "behavior",sh.behavior,"is_static",sh.isStatic,"z_bias",(int)sh.zBias,"lbp2_shape",sh.isLBP2Shape));
        }
        PGeneratedMesh gm=t.getPart(Part.GENERATED_MESH);
        if(gm!=null)r.put("generated_mesh",row("gfx_material",desc(gm.gfxMaterial),"bevel",desc(gm.bevel),
            "uv_offset",v4(gm.uvOffset),"plan_guid",guid(gm.planGUID),"visibility_flags",(int)gm.visibilityFlags,
            "uv_mode",gm.uvMode,"texture_scale",gm.textureScale,"no_bevel",gm.noBevel,"sharded",gm.sharded,
            "include_sides",gm.includeSides));
        PRef ref=t.getPart(Part.REF);
        if(ref!=null)r.put("ref",row("plan",desc(ref.plan),"strip_children",ref.stripChildren,"children_selectable",ref.childrenSelectable));
        return r;
    }

    static Map<String,Object> inspect(byte[] raw,boolean withScene)throws Exception {
        Map<String,Object> head=preflight(raw);
        SerializedResource res=new SerializedResource(raw);
        Revision rev=res.getRevision();
        if(rev.getHead()!=(int)head.get("head")||(rev.getBranchID()&0xffff)!=(int)head.get("branch_id"))
            throw new IOException("wrapper disagreement");
        MemoryInputStream stream=res.getStream();
        if(stream.getLength()>MAX_EXPANDED)throw new IOException("expanded budget");
        DepthSerializer s=new DepthSerializer(stream,rev);
        for(ResourceDescriptor d:res.getDependencies())s.addDependency(d);
        RLevel level=s.struct(null,RLevel.class);
        int consumed=s.getOffset(),length=stream.getLength();
        IdentityHashMap<Object,Integer> tag=new IdentityHashMap<>();List<Thing> things=new ArrayList<>();
        for(Map.Entry<Integer,Object> e:s.tags().entrySet()) {
            tag.put(e.getValue(),e.getKey());
            if(e.getValue() instanceof Thing)things.add((Thing)e.getValue());
        }
        Map<String,Long> partHist=new TreeMap<>(),meshes=new TreeMap<>(),shapeMat=new TreeMap<>(),gfx=new TreeMap<>(),
            bevels=new TreeMap<>(),refPlans=new TreeMap<>(),boneLen=new TreeMap<>(),polyVerts=new TreeMap<>();
        long rmCount=0,shapeCount=0,genCount=0,shapeGen=0,rmShape=0,posCount=0,shapeVertices=0;
        for(Thing t:things) {
            for(String p:parts(t))inc(partHist,p);
            PRenderMesh rm=t.getPart(Part.RENDER_MESH);PShape sh=t.getPart(Part.SHAPE);PGeneratedMesh gm=t.getPart(Part.GENERATED_MESH);
            if(t.hasPart(Part.POS))posCount++;
            if(rm!=null){rmCount++;inc(meshes,descKey(rm.mesh));inc(boneLen,Integer.toString(rm.boneThings==null?0:rm.boneThings.length));}
            if(sh!=null){shapeCount++;inc(shapeMat,descKey(sh.material));
                int n=sh.polygon==null||sh.polygon.vertices==null?0:sh.polygon.vertices.length;shapeVertices+=n;
                if(n>MAX_POLYGON_VERTICES)throw new IOException("polygon vertex budget");
                inc(polyVerts,n<=4?Integer.toString(n):n<=16?"5-16":n<=64?"17-64":n<=256?"65-256":">256");}
            if(gm!=null){genCount++;inc(gfx,descKey(gm.gfxMaterial));inc(bevels,descKey(gm.bevel));}
            if(sh!=null&&gm!=null)shapeGen++;
            if(rm!=null&&sh!=null)rmShape++;
            PRef ref=t.getPart(Part.REF);if(ref!=null)inc(refPlans,descKey(ref.plan));
        }
        Thing world=level.worldThing;PWorld pw=world==null?null:world.getPart(Part.WORLD);
        Map<String,Object> r=row("header",head,"toolkit_revision",rev.toString(),"sub_version",rev.getSubVersion(),
            "expanded_bytes",length,"consumed_bytes",consumed,"trailing_bytes",length-consumed,
            "cross_play_hash_count",level.crossPlayVitaDependencyHashes==null?0:level.crossPlayVitaDependencyHashes.length,
            "reference_tag_count",tag.size(),"thing_count",things.size(),"max_thing_depth",s.maxDepth,
            "world",row("uid",world==null?null:world.UID,"has_world_part",pw!=null,"world_things_listed",pw==null?0:pw.things.size(),
                "backdrop",thingRef(pw==null?null:pw.backdrop,tag),"backdrop_new",thingRef(pw==null?null:pw.backdropNew,tag),
                "world_parts",world==null?List.of():parts(world)),
            "part_histogram",partHist,
            "counts",row("pos",posCount,"render_mesh",rmCount,"shape",shapeCount,"generated_mesh",genCount,
                "shape_and_generated_mesh",shapeGen,"render_mesh_and_shape",rmShape,"shape_vertices_total",shapeVertices,
                "distinct_render_meshes",meshes.size(),"distinct_shape_materials",shapeMat.size(),
                "distinct_gfx_materials",gfx.size(),"distinct_bevels",bevels.size(),"distinct_ref_plans",refPlans.size()),
            "render_mesh_descriptors",meshes,"bone_things_length_histogram",boneLen,"shape_vertex_count_histogram",polyVerts,
            "shape_material_descriptors",shapeMat,"gfx_material_descriptors",gfx,"bevel_descriptors",bevels,"ref_plan_descriptors",refPlans);
        if(withScene) {
            List<Object> dump=new ArrayList<>();for(Thing t:things)dump.add(scene(t,tag));
            r.put("scene_things",dump);
        }
        return r;
    }

    // ---- authored synthetic guards (no game bytes) ----
    // payload is written verbatim after the compressed bool (callers include any chunk table).
    static byte[] header(int head,int branch,int flags,boolean compressed,byte[] payload,int deps) {
        ByteBuffer b=ByteBuffer.allocate(64+payload.length+4);
        b.put(new byte[]{'L','V','L','b'}).putInt(head);int depAt=b.position();b.putInt(0);
        if(head>=0x271)b.putShort((short)branch).putShort((short)0);
        if(head>=0x297)b.put((byte)flags);
        b.put((byte)(compressed?1:0));
        b.put(payload);int dep=b.position();b.putInt(deps);b.putInt(depAt,dep);
        return Arrays.copyOf(b.array(),b.position());
    }
    static byte[] stored(byte[] clear) {
        // zero-chunk envelope: Toolkit copies the bytes up to the dependency table
        ByteBuffer b=ByteBuffer.allocate(4+clear.length);b.putShort((short)1).putShort((short)0).put(clear);return b.array();
    }
    static byte[] ldHeader() {
        ByteBuffer b=ByteBuffer.allocate(32);
        b.put(new byte[]{'L','V','L','b'}).putInt(0x272).putInt(0).putShort((short)0x4c44).putShort((short)2).put((byte)7).put((byte)1);
        b.put(stored(new byte[]{5}));int dep=b.position();b.putInt(0);b.putInt(8,dep);
        return Arrays.copyOf(b.array(),b.position());
    }
    static void rejected(byte[] raw,String why)throws Exception {
        try{preflight(raw);}catch(IOException expected){return;}
        throw new AssertionError("accepted "+why);
    }
    static void selfTest()throws Exception {
        byte[] ok=header(0x3f8,0,7,true,stored(new byte[]{1,2,3}),0);
        Map<String,Object> h=preflight(ok);
        if(!h.get("head").equals(0x3f8)||!h.get("compression_flags").equals(7)||!h.get("declared_expanded_bytes").equals(3L))
            throw new AssertionError("synthetic header fields "+h);
        for(int n=0;n<ok.length;n++)rejected(Arrays.copyOf(ok,n),"truncation "+n);
        byte[] bad=ok.clone();bad[3]='e';rejected(bad,"encrypted");
        bad=ok.clone();bad[0]='P';rejected(bad,"wrong magic");
        rejected(header(0x3f9,0,7,true,stored(new byte[1]),0),"head above LBP2 max");
        rejected(header(0x3f8,0,7,true,stored(new byte[1]),-1),"negative dependency count");
        bad=header(0x3f8,0,7,true,stored(new byte[1]),0);bad[17]=2;rejected(bad,"compressed bool 2");
        // Declared chunk expansion above budget with tiny stored input.
        int chunks=0x7fff;ByteBuffer big=ByteBuffer.allocate(4+4*chunks);
        big.putShort((short)1).putShort((short)chunks);
        for(int i=0;i<chunks;i++)big.putShort((short)0).putShort((short)0xffff);
        rejected(header(0x3f8,0,7,true,big.array(),0),"expansion budget");
        // Chunk table claiming more stored bytes than present.
        ByteBuffer one=ByteBuffer.allocate(8);one.putShort((short)1).putShort((short)1).putShort((short)100).putShort((short)100);
        rejected(header(0x3f8,0,7,true,one.array(),0),"chunk overrun");
        // Leerdammer head 0x272 carries a flags byte when branch 0x4c44 revision >= 2 (checked via preflight fields).
        Map<String,Object> ld=preflight(ldHeader());
        if(!ld.get("branch_id").equals(0x4c44)||!ld.get("compression_flags").equals(7))throw new AssertionError("leerdammer header "+ld);
        // Uncompressed older head (no flags byte) still bounded.
        Map<String,Object> old=preflight(header(0x26e,0,0,false,new byte[]{9,9},2));
        if(!old.get("declared_expanded_bytes").equals(2L)||!old.get("dependency_count_raw").equals(2))throw new AssertionError("old head");
        // Deterministic projection helpers.
        if(!descKey(null).equals("null")||!guid(new GUID(0x1234)).equals("00001234"))throw new AssertionError("descriptor keys");
        System.out.println("Level synthetic header/truncation/encrypted/head/dependency/expansion guards passed");
    }

    static String sha1(byte[] b)throws Exception {return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-1").digest(b));}
    static boolean sameArchive(String sourcePath,Path archive) {
        String name=archive.getFileName().toString().toLowerCase(Locale.ROOT);
        return sourcePath.startsWith("USRDIR/")&&sourcePath.toLowerCase(Locale.ROOT).endsWith("/"+name);
    }

    static void run(String[] args)throws Exception {
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
                if(Thread.currentThread().isInterrupted()||System.nanoTime()>deadline)throw new IOException("oracle cancelled/time budget");
                JsonObject q=item.getAsJsonObject();String sha=q.get("sha1").getAsString();
                boolean withScene=q.has("scene")&&q.get("scene").getAsBoolean();
                Map<String,Object> r=row("sha1",sha,"map_path",q.get("map_path").getAsString(),"guid",q.get("guid").getAsString(),
                    "source_path",q.get("source_path").getAsString(),"stored_bytes",q.get("size").getAsLong(),"scene_requested",withScene);
                rows.add(r);
                try {
                    String src=q.get("source_path").getAsString();
                    List<Path> match=new ArrayList<>();for(Path a:archives)if(sameArchive(src,a))match.add(a);
                    if(match.size()!=1)throw new IOException("archive not supplied or ambiguous: "+src);
                    RandomAccessFile f=open.get(match.get(0));
                    if(f==null){f=new RandomAccessFile(match.get(0).toFile(),"r");open.put(match.get(0),f);}
                    long at=q.get("offset").getAsLong(),size=q.get("size").getAsLong();
                    if(at<0||size<12||size>MAX_STORED||at>f.length()-size||total+size>MAX_RUN_BYTES)throw new IOException("source/read budget");
                    byte[] raw=new byte[(int)size];f.seek(at);f.readFully(raw);total+=size;
                    if(!sha1(raw).equals(sha))throw new IOException("identifier mismatch");
                    r.putAll(inspect(raw,withScene));
                    r.put("status",((long)(int)r.get("trailing_bytes"))==0?"parsed_exact":"parsed_trailing_bytes");
                }catch(StackOverflowError e){r.putAll(row("status","failed","reason","StackOverflowError"));}
                catch(Exception e){
                    String msg=e.getMessage();if(msg!=null&&msg.length()>300)msg=msg.substring(0,300);
                    r.putAll(row("status",e instanceof IOException?"rejected":"failed","reason",e.getClass().getSimpleName()+": "+msg));
                }
                inc(status,(String)r.get("status"));
            }
        } finally {for(RandomAccessFile f:open.values())f.close();}
        Files.writeString(output,new GsonBuilder().serializeNulls().serializeSpecialFloatingPointValues().setPrettyPrinting().create().toJson(row(
            "schema_version",1,"toolkit_commit",PIN,"scope",SCOPE,"sample_bytes",total,"status_counts",status,
            "matrix_note","PPos world matrices are stored; local matrices are Toolkit fixup recomputations (head>=0x341)",
            "levels",rows)),StandardOpenOption.CREATE_NEW);
        System.out.println("Level oracle: "+rows.size()+" levels, "+total+" archived bytes, "+status);
    }
    public static void main(String[] args)throws Exception {
        Throwable[] failure=new Throwable[1];
        // Toolkit references recurse inline; a large dedicated stack keeps depth data-driven failures per level.
        Thread t=new Thread(null,()->{try{run(args);}catch(Throwable e){failure[0]=e;}},"level-oracle",1L<<30);
        t.start();t.join();
        if(failure[0]!=null){failure[0].printStackTrace();System.exit(1);}
    }
}
