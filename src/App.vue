<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { api, errorMessage, type FileRecord, type Grant, type Tag, type Status } from './api';
const files = ref<FileRecord[]>([]), tags = ref<Tag[]>([]), grants = ref<Grant[]>([]);
const selected = ref<FileRecord | null>(null);
const query = ref(''), extension = ref(''), filterTag = ref<number | null>(null);
const busy = ref(false), notice = ref(''), error = ref(''), settings = ref(false);
const tagDraft = ref(''), editingTag = ref<number | null>(null), renameDraft = ref('');
const confirmDelete = ref(false);
const allExtensions = ref<string[]>([]);
const activeGrants = computed(() => grants.value.filter(g => g.active).length);
const statusText: Record<Status, string> = { READY: '已登记', MISSING: '原文件缺失', UNAUTHORIZED: '读取未授权', CHANGED: '文件身份变化', ERROR: '无法读取' };
function size(n: number) { return n < 1024 ? `${n} B` : n < 1048576 ? `${(n / 1024).toFixed(1)} KB` : `${(n / 1048576).toFixed(1)} MB`; }
function date(n: number) { return new Date(n).toLocaleString('zh-CN', { hour12: false }); }
async function run(action: () => Promise<void>) {
  if (busy.value) return;
  busy.value = true; error.value = ''; notice.value = '';
  try { await action(); } catch (e) { error.value = errorMessage(e); }
  finally { busy.value = false; }
}
async function reload() {
  const [result, allTags, allGrants, allFiles] = await Promise.all([api.search(query.value, filterTag.value, extension.value), api.tags(), api.grants(), api.search()]);
  files.value = result; tags.value = allTags; grants.value = allGrants;
  allExtensions.value = [...new Set(allFiles.map(f => f.extension).filter(Boolean))].sort();
  if (selected.value) selected.value = allFiles.find(f => f.id === selected.value?.id) ?? null;
}
function search() { void run(reload); }
function chooseTag(id: number | null) { filterTag.value = id; settings.value = false; search(); }
function inspect(file: FileRecord) {
  confirmDelete.value = false;
  void run(async () => { selected.value = file; selected.value = await api.details(file.id); await reload(); });
}
function importFiles() {
  void run(async () => {
    const report = await api.importFiles(); await reload();
    notice.value = report.imported ? `已登记 ${report.imported} 个文件（重复文件已更新）。` : '未登记新文件。';
    error.value = report.errors.join('\n');
  });
}
function createTag() { void run(async () => { await api.createTag(tagDraft.value); tagDraft.value = ''; await reload(); }); }
function renameTag() { void run(async () => { if (editingTag.value !== null) await api.renameTag(editingTag.value, renameDraft.value); editingTag.value = null; await reload(); }); }
function setTag(tag: Tag, attached: boolean) {
  void run(async () => { if (selected.value) await api.setTag(selected.value.id, tag.id, attached); await reload(); });
}
function revoke(grant: Grant) { void run(async () => { await api.revoke(grant.id); await reload(); notice.value = '已撤销此项授权；其他重叠授权仍有效。索引和标签已保留。'; }); }
function grantDirectory() { void run(async () => { if (await api.authorizeDirectory()) notice.value = '目录已授权，不会扫描目录。'; await reload(); }); }
function relink() { void run(async () => { if (selected.value && await api.relink(selected.value.id)) notice.value = '身份校验通过，已重新关联原文件。'; await reload(); }); }
function deleteIndex() { void run(async () => { if (selected.value) await api.deleteIndex(selected.value.id); selected.value = null; confirmDelete.value = false; await reload(); notice.value = '索引已移除，原文件未修改。授权可在设置中单独撤销。'; }); }
onMounted(() => { void run(reload); });
</script>

<template>
  <div class="app-shell" :aria-busy="busy">
    <aside class="sidebar">
      <div class="brand"><span class="brand-mark">▱</span><div>Personal File Tags<small>你的本地文件标签库</small></div></div>
      <div class="local-note"><span class="dot"></span> 本地存储 · 无云端连接</div>
      <button class="nav-item" :class="{ active: !settings && filterTag === null }" @click="chooseTag(null)" :disabled="busy"><span>▤</span> 全部文件</button>
      <div class="section-label">标签 <span>{{ tags.length }}</span></div>
      <div class="tag-nav">
        <div v-for="tag in tags" :key="tag.id" class="tag-row">
          <button class="nav-item" :class="{ active: !settings && filterTag === tag.id }" @click="chooseTag(tag.id)" :disabled="busy"><span class="hash">#</span><span class="ellipsis">{{ tag.name }}</span></button>
          <button class="icon-button" :aria-label="`重命名标签 ${tag.name}`" :disabled="busy" @click="editingTag = tag.id; renameDraft = tag.name">✎</button>
        </div>
        <p v-if="!tags.length" class="muted small">创建第一个标签，开始整理。</p>
      </div>
      <form class="new-tag" @submit.prevent="createTag"><input v-model="tagDraft" aria-label="新标签名称" maxlength="64" placeholder="新标签名称" :disabled="busy"><button :disabled="busy || !tagDraft.trim()" aria-label="创建标签">＋</button></form>
      <button class="nav-item settings-button" :class="{ active: settings }" @click="settings = true" :disabled="busy"><span>⚙</span> 设置与授权 <small>{{ activeGrants }}</small></button>
      <div class="version">V0.1 · 只登记，不搬动文件</div>
    </aside>
    <main>
      <header><div><div class="eyebrow">PERSONAL LIBRARY</div><h1>{{ settings ? '设置与授权' : '文件资料库' }}</h1><p>{{ settings ? '管理应用可以按需读取的位置。' : '保留原来的位置，用标签建立新的秩序。' }}</p></div><button class="primary" @click="importFiles" :disabled="busy">＋ 登记文件</button></header>
      <div v-if="error" class="banner error" role="alert">{{ error }}</div>
      <div v-if="notice" class="banner success" role="status">{{ notice }}</div>
      <div v-if="busy" class="working" role="status">正在处理，请稍候…</div>
      <section v-if="settings" class="settings-view">
        <div class="settings-card"><h2>文件访问授权</h2><p>登记文件即授权后续查看详情时读取该文件的元数据。目录授权适用于其内部已登记文件，不会自动遍历或扫描。</p><button @click="grantDirectory" :disabled="busy">＋ 选择并授权目录</button><p class="small muted">文件授权与目录授权独立。要停止某文件的后续读取，请撤销覆盖它的所有有效授权。撤销会保留已有索引和标签。</p></div>
        <div class="grant-list"><div v-for="grant in grants" :key="grant.id" class="grant"><span class="kind">{{ grant.kind === 'FILE' ? '文件' : '目录' }}</span><div class="grant-path">{{ grant.path }}<small>{{ grant.active ? '有效授权' : '已撤销' }}</small></div><button v-if="grant.active" @click="revoke(grant)" :disabled="busy">撤销</button></div><p v-if="!grants.length" class="empty-line">尚未授权任何文件或目录。</p></div>
        <div class="settings-card"><h2>本地数据与边界</h2><p>SQLite 数据保存在当前 Windows 用户的应用数据目录。应用仅保存路径、元数据、授权与标签，不读取文件正文、不上传数据，也不在原文件目录生成附属文件。</p><p class="small muted">V0.1 保守拒绝网络路径、符号链接、junction 和其他 reparse point。数据库未加密；应用以当前用户权限运行，不是操作系统级沙箱。</p></div>
      </section>
      <template v-else>
        <form class="toolbar" @submit.prevent="search"><div class="search"><span>⌕</span><input v-model="query" aria-label="按文件名搜索" maxlength="256" placeholder="按文件名搜索…" :disabled="busy"></div><select v-model="extension" aria-label="文件类型筛选" :disabled="busy" @change="search"><option value="">全部类型</option><option v-for="ext in allExtensions" :key="ext" :value="ext">.{{ ext }}</option></select><button :disabled="busy">搜索</button></form>
        <div class="results-heading"><span>{{ filterTag ? `标签：${tags.find(t => t.id === filterTag)?.name ?? ''}` : '全部文件' }}</span><span>{{ files.length }} 个结果 · 状态在查看详情时检查</span></div>
        <div class="workspace">
          <section class="file-list" aria-label="文件列表">
            <div v-if="!files.length" class="empty"><div class="empty-icon">▱</div><h2>{{ query || filterTag || extension ? '没有匹配的文件' : '从你的第一个文件开始' }}</h2><p>主动选择文件，登记原位置，再添加标签。</p><button @click="importFiles" :disabled="busy">选择文件</button></div>
            <button v-for="file in files" :key="file.id" class="file-card" :class="{ selected: selected?.id === file.id }" @click="inspect(file)" :disabled="busy"><span class="file-icon">{{ file.extension.slice(0, 4).toUpperCase() || 'FILE' }}</span><div class="file-info"><strong>{{ file.name }}</strong><span class="path" :title="file.path">{{ file.path }}</span><div class="chips"><span v-for="tag in file.tags" :key="tag.id" class="chip">{{ tag.name }}</span><span v-if="!file.tags.length" class="muted small">尚未添加标签</span></div></div><div class="file-meta"><span>{{ size(file.size) }}</span><span class="status" :class="file.status">{{ statusText[file.status] }}</span></div></button>
          </section>
          <aside class="details" aria-label="文件详情">
            <template v-if="selected"><div class="section-label">文件详情</div><h2>{{ selected.name }}</h2><span class="status" :class="selected.status">{{ selected.status }} · {{ statusText[selected.status] }}</span><dl><dt>原始位置</dt><dd class="full-path">{{ selected.path }}</dd><dt>文件类型 / 大小</dt><dd>{{ selected.extension || '无扩展名' }} · {{ size(selected.size) }}</dd><dt>最后修改</dt><dd>{{ date(selected.modified_at) }}</dd><dt>登记时间</dt><dd>{{ date(selected.registered_at) }}</dd></dl><div class="section-label">人工标签</div><div class="tag-options"><label v-for="tag in tags" :key="tag.id"><input type="checkbox" :checked="selected.tags.some(t => t.id === tag.id)" :disabled="busy" @change="setTag(tag, ($event.target as HTMLInputElement).checked)">{{ tag.name }}</label><p v-if="!tags.length" class="muted small">先在左侧创建标签。</p></div><div class="detail-actions"><button @click="inspect(selected)" :disabled="busy">检查文件状态</button><button @click="relink" :disabled="busy">重新选择原文件关联</button><button class="text-danger" @click="confirmDelete = true" :disabled="busy">移除此索引…</button></div><div v-if="confirmDelete" class="confirm-box"><p>仅移除该索引及标签关联。原文件、标签词表和授权不会删除。</p><button class="danger" @click="deleteIndex" :disabled="busy">确认移除索引</button><button @click="confirmDelete = false" :disabled="busy">取消</button></div></template>
            <div v-else class="detail-empty"><span>◇</span><h2>选择一个文件</h2><p>查看原始位置、文件状态与标签。</p></div>
          </aside>
        </div>
      </template>
    </main>
    <div v-if="editingTag !== null" class="modal-backdrop"><form class="modal" role="dialog" aria-modal="true" aria-labelledby="rename-title" @submit.prevent="renameTag"><h2 id="rename-title">重命名标签</h2><input v-model="renameDraft" aria-label="标签名称" maxlength="64" :disabled="busy" autofocus><div><button type="button" @click="editingTag = null" :disabled="busy">取消</button><button class="primary" :disabled="busy || !renameDraft.trim()">保存</button></div></form></div>
  </div>
</template>
