# Below is outdated, will be updated soon, thanks.


# 🛠️ Software Design Document for SUSE EDGE Support Software ϟϟ

## 1. 🎯 Introduction  

### 📌 Purpose  
This application is designed to streamline support engineering tasks by automating data collection and log gathering from customer environments. It will allow customers to:  
✅ Trigger execution  
✅ Input relevant case details  
✅ Upload collected data to the Salesforce (SF) case or generate a ZIP file for manual submission  

### 📌 Scope  
- 🛠️ Part of the EDGE release, executable by customers  
- 📂 Collects system logs, configurations, and diagnostics from Kubernetes clusters  
- 🌐 Provides a web-based UI  
- 🏗️ Runs in a container within the default namespace but has access to all namespaces  

### 👥 Stakeholders  
- 🔹 **Support Engineers:** Maintain and enhance the application & Use collected data for debugging and issue resolution  
- 🔹 **Customers:** Trigger execution and provide input data  

---

## 2. 🏗️ System Overview  

### 🏷️ High-Level Description  
The application will offer a simple web interface for customers to select log and config options, input case details, and initiate data collection. The data can be uploaded to Salesforce or downloaded as a ZIP.  

### 🔑 Key Features  
✅ Visibility into installed EDGE components  
✅ Selection of logs and configurations for collection  
✅ Manual log file and command output gathering  
✅ Configurable compression options  
✅ Integration with Salesforce for automatic case updates  
✅ Kubernetes cluster metrics collection via Kubernetes metric server

### ⚖️ Assumptions & Constraints  
- 📦 Must be containerized  
- 🔍 Must access all namespaces in the cluster  

---

## 3. 🏗️ Architecture & Design  

### 🛠️ Technology Stack  
- **Backend:** client-go (Golang Kubernetes client library)  
- **Frontend:** ??? (Maybe React or Vue.js)  
- **Containerization:** Podman
- **Orchestration:** Kubernetes  
- **Storage:** Local (ZIP files), Salesforce API for case uploads  
- **Authentication:** Kubernetes RBAC  

### 🏛️ High-Level Architecture  
📌 **Web UI** → Communicates with API Server  
📌 **API Server** → Orchestrates log collection and data packaging  
📌 **Kubernetes API** → Retrieves logs, configurations, and metrics  

### 🔧 Component Breakdown  
- **🎨 UI Component:** Web-based interface  
- **📡 API Component:** Handles user requests, log retrieval, and data processing  
- **📥 Data Collector:** Gathers logs, configurations, and metrics  
- **📦 Compression & Upload Module:** Handles data packaging and transmission  

---

## 4. 📂 Data Model & Storage  

### 🔄 Data Flow  
1️⃣ User selects logs and configurations to collect  
2️⃣ API triggers data collection from Kubernetes  
3️⃣ Data is packaged and either uploaded to Salesforce or made available for download  

### 📑 Storage 
- ⏳ Temporary storage for ZIP files before upload  

---

## 5. 📡 API Design  

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/components` | List installed EDGE components |
| POST | `/collect` | Trigger data collection with user-specified options |
| GET | `/status` | Check progress of data collection |
| POST | `/upload` | Upload collected data to Salesforce |

---

## 6. 🎨 User Interface  
- 🌐 Web-based UI for selection and execution  
- 📝 Simple forms for data input (case number, customer details)  

---

## 7. ⚠️ Error Handling & Logging  
- 📜 Logging to Kubernetes logs (maybe also file on disk?)  
- 🚦 Error handling with structured responses  

---

## 8. 🚀 Deployment & Maintenance  
- 🏗️ Deployed as a Kubernetes container in the default namespace  
- 🔄 CI/CD pipeline for updates  (maybe ?)
- 📌 Versioned releases following the SUSE EDGE release cycle  

---

## 9. 🧪 Testing Strategy  
✅ Tests for API endpoints  
✅ Tests for Kubernetes API and Salesforce API integration  
✅ UI testing (ask Jiri ?)

---

## 10. 🔮 Future Considerations  
🔹 Extensibility for additional log sources      
🔹 Support for additional compression formats    
🔹 Improved UI/UX for better usability   
🔹 Compatibility with future Kubernetes versions  
🔹 Additional security enhancements such as encryption and audit logging  

---

📌 **End of Document** ✅
